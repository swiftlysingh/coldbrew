//! Package installation

use crate::cli::output::{format_duration, Output};
use crate::config::GlobalConfig;
use crate::core::package::{InstalledPackage, PackageMetadata, RuntimeDependency};
use crate::core::{BottleFile, DependencyResolver, Formula, Platform};
use crate::error::{ColdbrewError, Result};
use crate::ops::verify;
use crate::registry::{GhcrClient, Index};
use crate::storage::{Cache, Cellar, Database, Paths, ShimManager, Store};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

struct InstallContext<'a> {
    paths: &'a Paths,
    cache: &'a Cache,
    store: &'a Store,
    shim_manager: &'a ShimManager,
    ghcr: &'a GhcrClient,
    platform: &'a Platform,
    cellar: &'a Cellar,
    output: &'a Output,
}

struct InstallOptions<'a> {
    installed_as_dependency: bool,
    installed_for: Option<&'a str>,
    force: bool,
}

/// Install a package
pub async fn install(
    paths: &Paths,
    name: &str,
    version: Option<&str>,
    skip_deps: bool,
    force: bool,
    output: &Output,
) -> Result<InstalledPackage> {
    let config = GlobalConfig::load(paths)?;
    let parallel_downloads = config.settings.parallel_downloads.max(1);
    let cdn_racing = config.settings.cdn_racing;

    let index = Index::new(paths.clone());
    let cellar = Cellar::new(paths.clone());
    let cache = Arc::new(Cache::new(paths.clone()));
    let store = Store::new(paths.clone());
    let shim_manager = ShimManager::new(paths.clone());
    let ghcr = Arc::new(GhcrClient::new_with_options(cdn_racing)?);
    let platform = Platform::detect()?;
    let ctx = InstallContext {
        paths,
        cache: cache.as_ref(),
        store: &store,
        shim_manager: &shim_manager,
        ghcr: ghcr.as_ref(),
        platform: &platform,
        cellar: &cellar,
        output,
    };

    let formulas = index.list_formulas()?;
    let mut formula_map = HashMap::new();
    for formula in formulas {
        formula_map.insert(formula.name.clone(), formula);
    }

    let root_formula = formula_map
        .get(name)
        .ok_or_else(|| ColdbrewError::PackageNotFound(name.to_string()))?;

    let install_order: Vec<String> = if skip_deps {
        vec![name.to_string()]
    } else {
        let mut resolver = DependencyResolver::new();
        resolver.add_formulas(formula_map.values().cloned());
        resolver
            .resolve(name)?
            .into_iter()
            .map(|dep| dep.name)
            .collect()
    };

    if !skip_deps && !root_formula.dependencies.is_empty() {
        output.info(&format!(
            "Installing {} dependencies...",
            root_formula.dependencies.len()
        ));
    }

    let download_groups =
        plan_download_groups(&ctx, &install_order, &formula_map, name, version, force)?;

    let mut download_manager = if !download_groups.is_empty() {
        output.info(&format!(
            "Downloading {} bottles ({} parallel)",
            download_groups.len(),
            parallel_downloads
        ));
        cache.init()?;
        Some(DownloadManager::start(
            cache.clone(),
            ghcr.clone(),
            download_groups,
            parallel_downloads,
            output,
        ))
    } else {
        None
    };

    let mut installed_versions: HashMap<String, String> = HashMap::new();
    let mut root_installed: Option<InstalledPackage> = None;

    for pkg_name in install_order {
        let formula = formula_map
            .get(&pkg_name)
            .ok_or_else(|| ColdbrewError::PackageNotFound(pkg_name.clone()))?;

        let is_root = pkg_name == name;

        if !is_root {
            if let Some(existing) = cellar.latest_version(&pkg_name)? {
                output.debug(&format!(
                    "Dependency {} already installed at {}",
                    pkg_name, existing
                ));
                installed_versions.insert(pkg_name.clone(), existing);
                continue;
            }
        }

        let target_version = if is_root {
            version.unwrap_or(&formula.versions.stable)
        } else {
            &formula.versions.stable
        };

        if is_root && cellar.is_installed(&pkg_name, target_version) && !force {
            return Err(ColdbrewError::PackageAlreadyInstalled {
                name: pkg_name.clone(),
                version: target_version.to_string(),
            });
        }

        let runtime_deps = if is_root && skip_deps {
            Vec::new()
        } else {
            resolve_runtime_deps(formula, &installed_versions, &cellar, paths)?
        };

        let bottle_plan = resolve_bottle(formula, ctx.platform, &pkg_name)?;

        if let Some(manager) = download_manager.as_mut() {
            manager.wait_for(&bottle_plan.file.sha256).await?;
        }

        let options = InstallOptions {
            installed_as_dependency: !is_root,
            installed_for: if is_root { None } else { Some(name) },
            force: is_root && force,
        };
        let installed = install_single(
            &ctx,
            &pkg_name,
            target_version,
            formula,
            bottle_plan,
            runtime_deps,
            options,
        )
        .await?;

        installed_versions.insert(pkg_name.clone(), installed.version.clone());

        if is_root {
            root_installed = Some(installed);
        }
    }

    if let Some(manager) = download_manager.as_mut() {
        manager.wait_for_all().await?;
    }

    root_installed.ok_or_else(|| ColdbrewError::PackageNotFound(name.to_string()))
}

/// Install from a lockfile
pub async fn install_from_lockfile(
    paths: &Paths,
    lockfile: &crate::config::Lockfile,
    output: &Output,
) -> Result<Vec<InstalledPackage>> {
    let mut installed = Vec::new();

    for (name, locked) in &lockfile.packages {
        output.info(&format!("Installing {} {}...", name, locked.version));

        let pkg = install(
            paths,
            name,
            Some(&locked.version),
            true, // Skip deps, lockfile has them
            false,
            output,
        )
        .await?;

        installed.push(pkg);
    }

    Ok(installed)
}

fn resolve_runtime_deps(
    formula: &Formula,
    installed_versions: &HashMap<String, String>,
    cellar: &Cellar,
    paths: &Paths,
) -> Result<Vec<RuntimeDependency>> {
    let mut runtime_deps = Vec::new();

    for dep_name in &formula.dependencies {
        let version = if let Some(version) = installed_versions.get(dep_name) {
            version.clone()
        } else if let Some(version) = cellar.latest_version(dep_name)? {
            version
        } else {
            return Err(ColdbrewError::DependencyResolutionFailed {
                package: formula.name.clone(),
                dep: dep_name.clone(),
            });
        };

        runtime_deps.push(RuntimeDependency {
            name: dep_name.clone(),
            version: version.clone(),
            path: paths.cellar_package(dep_name, &version),
        });
    }

    Ok(runtime_deps)
}

#[derive(Clone)]
struct BottlePlan {
    tag: String,
    file: BottleFile,
}

fn resolve_bottle(formula: &Formula, platform: &Platform, package: &str) -> Result<BottlePlan> {
    let bottle_tags = platform.bottle_tags();
    let (bottle_tag, bottle_file) = formula
        .bottle
        .stable
        .as_ref()
        .and_then(|stable| stable.best_for_platform(&bottle_tags))
        .ok_or_else(|| ColdbrewError::NoBottleAvailable {
            package: package.to_string(),
            platform: platform.bottle_tag(),
        })?;
    Ok(BottlePlan {
        tag: bottle_tag,
        file: bottle_file.clone(),
    })
}

struct DownloadGroup {
    sha256: String,
    name: String,
    version: String,
    tag: String,
    formula: Formula,
    bottle_file: BottleFile,
    dest: PathBuf,
}

struct DownloadHandle {
    handle: JoinHandle<Result<()>>,
}

struct DownloadManager {
    handles: HashMap<String, DownloadHandle>,
    progress: Option<Arc<DownloadProgress>>,
}

struct DownloadProgress {
    bar: ProgressBar,
    total: usize,
    completed: AtomicUsize,
}

impl DownloadProgress {
    fn new(total: usize, message: &str) -> Option<Arc<Self>> {
        if total == 0 {
            return None;
        }

        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
                .unwrap(),
        );
        bar.set_message(message.to_string());

        Some(Arc::new(Self {
            bar,
            total,
            completed: AtomicUsize::new(0),
        }))
    }

    fn mark_complete(&self) {
        let done = self.completed.fetch_add(1, Ordering::SeqCst) + 1;
        self.bar.set_position(done as u64);
        if done >= self.total {
            self.bar.finish_and_clear();
        }
    }
}

impl DownloadManager {
    fn start(
        cache: Arc<Cache>,
        ghcr: Arc<GhcrClient>,
        groups: Vec<DownloadGroup>,
        parallel_downloads: usize,
        output: &Output,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(parallel_downloads.max(1)));
        let progress = DownloadProgress::new(groups.len(), "Downloading bottles");
        let mut handles = HashMap::new();

        for group in groups {
            let sha256 = group.sha256.clone();
            let cache = cache.clone();
            let ghcr = ghcr.clone();
            let semaphore = semaphore.clone();
            let progress = progress.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire_owned().await.map_err(|err| {
                    ColdbrewError::Other(format!("Failed to acquire download slot: {}", err))
                })?;
                let result = download_group(ghcr.as_ref(), cache.as_ref(), group).await;
                if let Some(progress) = progress.as_ref() {
                    progress.mark_complete();
                }
                result
            });

            handles.insert(sha256, DownloadHandle { handle });
        }

        if handles.is_empty() {
            output.debug("No downloads needed");
        }

        Self { handles, progress }
    }

    async fn wait_for(&mut self, sha256: &str) -> Result<()> {
        if let Some(handle) = self.handles.remove(sha256) {
            handle
                .handle
                .await
                .map_err(|err| ColdbrewError::Other(err.to_string()))??;
        }
        Ok(())
    }

    async fn wait_for_all(&mut self) -> Result<()> {
        let keys: Vec<String> = self.handles.keys().cloned().collect();
        for key in keys {
            self.wait_for(&key).await?;
        }
        if let Some(progress) = self.progress.as_ref() {
            progress.bar.finish_and_clear();
        }
        Ok(())
    }
}

fn plan_download_groups(
    ctx: &InstallContext<'_>,
    install_order: &[String],
    formula_map: &HashMap<String, Formula>,
    root: &str,
    root_version: Option<&str>,
    force: bool,
) -> Result<Vec<DownloadGroup>> {
    let mut groups: HashMap<String, DownloadGroup> = HashMap::new();

    for pkg_name in install_order {
        let formula = formula_map
            .get(pkg_name)
            .ok_or_else(|| ColdbrewError::PackageNotFound(pkg_name.clone()))?;
        let is_root = pkg_name == root;

        if !is_root && ctx.cellar.latest_version(pkg_name)?.is_some() {
            continue;
        }

        let target_version = if is_root {
            root_version.unwrap_or(&formula.versions.stable)
        } else {
            &formula.versions.stable
        };

        if is_root && ctx.cellar.is_installed(pkg_name, target_version) && !force {
            return Err(ColdbrewError::PackageAlreadyInstalled {
                name: pkg_name.clone(),
                version: target_version.to_string(),
            });
        }

        let bottle_plan = resolve_bottle(formula, ctx.platform, pkg_name)?;

        let sha256 = bottle_plan.file.sha256.clone();
        if ctx.cache.is_cached(&sha256) {
            continue;
        }

        let dest = ctx.cache.blob_path(&sha256);

        if groups.contains_key(&sha256) {
            continue;
        }

        groups.insert(
            sha256.clone(),
            DownloadGroup {
                sha256,
                name: pkg_name.clone(),
                version: target_version.to_string(),
                tag: bottle_plan.tag.clone(),
                formula: formula.clone(),
                bottle_file: bottle_plan.file.clone(),
                dest,
            },
        );
    }

    Ok(groups.into_values().collect())
}

async fn download_group(ghcr: &GhcrClient, cache: &Cache, group: DownloadGroup) -> Result<()> {
    if group.dest.exists() {
        let size = group.dest.metadata().map(|meta| meta.len()).unwrap_or(0);
        cache.record_blob_metadata(
            &group.sha256,
            Some(&group.name),
            Some(&group.version),
            Some(&group.tag),
            size,
        )?;
        return Ok(());
    }

    if let Some(parent) = group.dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = cache.blob_temp_path(&group.sha256);

    if temp_path.exists() {
        let _ = fs::remove_file(&temp_path);
    }

    ghcr.download_bottle(&group.formula, &group.bottle_file, &temp_path, |_, _| {})
        .await?;

    fs::rename(&temp_path, &group.dest)?;
    let size = group.dest.metadata().map(|meta| meta.len()).unwrap_or(0);
    cache.record_blob_metadata(
        &group.sha256,
        Some(&group.name),
        Some(&group.version),
        Some(&group.tag),
        size,
    )?;

    Ok(())
}

async fn install_single(
    ctx: &InstallContext<'_>,
    name: &str,
    version: &str,
    formula: &Formula,
    bottle_plan: BottlePlan,
    runtime_deps: Vec<RuntimeDependency>,
    options: InstallOptions<'_>,
) -> Result<InstalledPackage> {
    let install_start = Instant::now();
    if ctx.cellar.is_installed(name, version) && !options.force {
        return Err(ColdbrewError::PackageAlreadyInstalled {
            name: name.to_string(),
            version: version.to_string(),
        });
    }

    ctx.output
        .debug(&format!("Using bottle tag: {}", bottle_plan.tag));

    let bottle_path = if let Some(cached) = ctx.cache.get_cached(&bottle_plan.file.sha256) {
        ctx.output.debug("Using cached bottle");
        cached
    } else {
        ctx.output.debug("Downloading bottle...");
        let download_start = Instant::now();
        let download_path = ctx.cache.blob_temp_path(&bottle_plan.file.sha256);
        let final_path = ctx.cache.blob_path(&bottle_plan.file.sha256);

        if let Some(parent) = download_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if download_path.exists() {
            let _ = std::fs::remove_file(&download_path);
        }

        let pb = ctx
            .output
            .download_progress(0, &format!("Downloading {}", name));

        ctx.ghcr
            .download_bottle(
                formula,
                &bottle_plan.file,
                &download_path,
                |downloaded, total| {
                    if total > 0 {
                        pb.set_length(total);
                    }
                    pb.set_position(downloaded);
                },
            )
            .await?;

        pb.finish_and_clear();

        std::fs::rename(&download_path, &final_path)?;
        let size = final_path.metadata().map(|meta| meta.len()).unwrap_or(0);
        ctx.cache.record_blob_metadata(
            &bottle_plan.file.sha256,
            Some(name),
            Some(version),
            Some(&bottle_plan.tag),
            size,
        )?;

        ctx.output.debug(&format!(
            "Downloaded {} in {}",
            name,
            format_duration(download_start.elapsed().as_secs())
        ));

        final_path
    };

    let mut store_entry = None;
    for attempt in 1..=2 {
        ctx.output.debug("Verifying checksum...");
        if let Err(err) = verify::verify_bottle(&bottle_path, &bottle_plan.file.sha256, name) {
            if attempt < 2 {
                ctx.output
                    .debug("Checksum mismatch, re-downloading bottle...");
                ctx.cache.remove(&bottle_plan.file.sha256)?;
                let download_path = ctx.cache.blob_temp_path(&bottle_plan.file.sha256);
                if let Some(parent) = download_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if download_path.exists() {
                    let _ = std::fs::remove_file(&download_path);
                }
                ctx.ghcr
                    .download_bottle(formula, &bottle_plan.file, &download_path, |_, _| {})
                    .await?;
                std::fs::rename(&download_path, &bottle_path)?;
                let size = bottle_path.metadata().map(|meta| meta.len()).unwrap_or(0);
                ctx.cache.record_blob_metadata(
                    &bottle_plan.file.sha256,
                    Some(name),
                    Some(version),
                    Some(&bottle_plan.tag),
                    size,
                )?;
                continue;
            }
            return Err(err);
        }

        let store_start = Instant::now();
        ctx.output.debug("Extracting to store...");
        match ctx
            .store
            .ensure_entry(&bottle_plan.file.sha256, &bottle_path)
        {
            Ok(entry) => {
                store_entry = Some(entry);
                ctx.output.debug(&format!(
                    "Store entry ready in {}",
                    format_duration(store_start.elapsed().as_secs())
                ));
                break;
            }
            Err(err) => {
                if attempt < 2 {
                    ctx.output
                        .debug("Store extraction failed, re-downloading bottle...");
                    ctx.cache.remove(&bottle_plan.file.sha256)?;
                    continue;
                }
                return Err(err);
            }
        }
    }

    let store_entry = store_entry.ok_or_else(|| {
        ColdbrewError::ExtractionFailed("Failed to extract store entry".to_string())
    })?;

    let materialize_start = Instant::now();
    ctx.output.debug("Materializing to cellar...");
    let install_path = ctx
        .store
        .materialize(&bottle_plan.file.sha256, name, version)?;
    ctx.output.debug(&format!(
        "Materialized {} in {}",
        name,
        format_duration(materialize_start.elapsed().as_secs())
    ));

    let db = Database::new(ctx.paths.clone());
    let conn = db.connect()?;
    db.upsert_store_entry(&conn, &bottle_plan.file.sha256, store_entry.size_bytes)?;
    db.add_store_ref(&conn, &bottle_plan.file.sha256, name, version)?;

    let mut installed = InstalledPackage::new(
        name.to_string(),
        version.to_string(),
        formula.tap.clone(),
        install_path,
    );
    installed.runtime_dependencies = runtime_deps;
    installed.bottle_tag = Some(bottle_plan.tag);
    installed.bottle_sha256 = Some(bottle_plan.file.sha256.clone());
    installed.keg_only = formula.keg_only;
    installed.caveats = formula.caveats.clone();
    installed.installed_as_dependency = options.installed_as_dependency;
    if let Some(installed_for) = options.installed_for {
        installed.installed_for = Some(installed_for.to_string());
    }

    let binaries = ctx.cellar.get_binaries(name, version)?;
    installed.binaries = binaries.clone();

    if !formula.keg_only && !binaries.is_empty() {
        ctx.output
            .debug(&format!("Creating shims for {} binaries", binaries.len()));
        ctx.shim_manager.create_shims(name, version, &binaries)?;
        installed.linked = true;
    }

    let metadata = PackageMetadata::new(installed.clone(), bottle_plan.file.url.clone());
    ctx.cellar.save_metadata(&metadata)?;

    ctx.output.debug(&format!(
        "Installed {} {} in {}",
        name,
        version,
        format_duration(install_start.elapsed().as_secs())
    ));

    Ok(installed)
}
