//! Package installation

use crate::cli::output::{format_bytes, format_duration, Output};
use crate::config::GlobalConfig;
use crate::core::package::{InstalledPackage, PackageMetadata, RuntimeDependency};
use crate::core::platform::Os;
use crate::core::version::{version_matches, Version};
use crate::core::{BottleFile, DependencyResolver, Formula, Platform};
use crate::error::{ColdbrewError, Result};
use crate::ops::relocate;
use crate::ops::verify;
use crate::registry::{GhcrClient, Index};
use crate::storage::{Cache, Cellar, Database, Paths, ShimManager, Store};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
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
    download_semaphore: Arc<Semaphore>,
    extract_semaphore: Arc<Semaphore>,
    codesign_semaphore: Arc<Semaphore>,
}

struct InstallOptions<'a> {
    installed_as_dependency: bool,
    installed_for: Option<&'a str>,
    force: bool,
}

#[derive(Clone, Copy, Debug)]
enum VersionPolicy {
    Flexible,
    Exact,
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
    install_with_policy(
        paths,
        name,
        version,
        skip_deps,
        force,
        VersionPolicy::Flexible,
        output,
    )
    .await
}

async fn install_with_policy(
    paths: &Paths,
    name: &str,
    version: Option<&str>,
    skip_deps: bool,
    force: bool,
    version_policy: VersionPolicy,
    output: &Output,
) -> Result<InstalledPackage> {
    let config = GlobalConfig::load(paths)?;
    let cdn_racing = config.settings.cdn_racing;
    let download_semaphore = Arc::new(Semaphore::new(config.settings.parallel_downloads.max(1)));
    let extract_semaphore = Arc::new(Semaphore::new(config.settings.parallel_extractions.max(1)));
    let codesign_semaphore = Arc::new(Semaphore::new(config.settings.parallel_codesigning.max(1)));

    let index = Index::new(paths.clone());
    let cellar = Cellar::new(paths.clone());
    let cache = Arc::new(Cache::new(paths.clone()));
    let store = Arc::new(Store::new(paths.clone()));
    let shim_manager = ShimManager::new(paths.clone());
    let ghcr = Arc::new(GhcrClient::new_with_options(cdn_racing)?);
    let platform = Platform::detect()?;
    let ctx = InstallContext {
        paths,
        cache: cache.as_ref(),
        store: store.as_ref(),
        shim_manager: &shim_manager,
        ghcr: ghcr.as_ref(),
        platform: &platform,
        cellar: &cellar,
        output,
        download_semaphore,
        extract_semaphore,
        codesign_semaphore,
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

    let download_groups = plan_download_groups(
        &ctx,
        &install_order,
        &formula_map,
        name,
        version,
        force,
        version_policy,
    )?;
    let parallel_downloads = if download_groups.is_empty() {
        1
    } else {
        std::cmp::min(
            config.settings.parallel_downloads.max(1),
            download_groups.len(),
        )
    };

    let mut download_manager = if !download_groups.is_empty() {
        output.info(&format!(
            "Downloading {} bottles ({} parallel)",
            download_groups.len(),
            parallel_downloads
        ));
        cache.init()?;
        Some(DownloadManager::start(
            cache.clone(),
            store.clone(),
            ghcr.clone(),
            download_groups,
            ctx.download_semaphore.clone(),
            ctx.extract_semaphore.clone(),
            paths.clone(),
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
            match version {
                Some(requested) => {
                    resolve_requested_version(&pkg_name, requested, formula, version_policy)?
                }
                None => formula.version_with_revision(),
            }
        } else {
            formula.version_with_revision()
        };

        if is_root && cellar.is_installed(&pkg_name, &target_version) && !force {
            return Err(ColdbrewError::PackageAlreadyInstalled {
                name: pkg_name.clone(),
                version: target_version.clone(),
            });
        }

        let runtime_deps = if is_root && skip_deps {
            Vec::new()
        } else {
            resolve_runtime_deps(formula, &installed_versions, &cellar, paths)?
        };

        let bottle_plan = resolve_bottle(formula, ctx.platform, &pkg_name)?;

        if let Some(manager) = download_manager.as_mut() {
            manager.wait_for(&bottle_plan.file.sha256, output).await?;
        }

        let options = InstallOptions {
            installed_as_dependency: !is_root,
            installed_for: if is_root { None } else { Some(name) },
            force: is_root && force,
        };
        let installed = install_single(
            &ctx,
            &pkg_name,
            &target_version,
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
        manager.wait_for_all(output).await?;
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

        let pkg = install_with_policy(
            paths,
            name,
            Some(&locked.version),
            true, // Skip deps, lockfile has them
            false,
            VersionPolicy::Exact,
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

fn resolve_requested_version(
    name: &str,
    requested: &str,
    formula: &Formula,
    policy: VersionPolicy,
) -> Result<String> {
    let resolved = formula.version_with_revision();

    if matches!(policy, VersionPolicy::Exact) {
        if requested == resolved {
            return Ok(resolved);
        }

        return Err(ColdbrewError::VersionNotAvailable {
            name: name.to_string(),
            requested: requested.to_string(),
            available: resolved,
        });
    }

    if requested == resolved {
        return Ok(resolved);
    }

    if requested == formula.versions.stable {
        return Ok(resolved);
    }

    match Version::parse(&resolved) {
        Ok(resolved_version) => {
            if version_matches(&resolved_version, requested) {
                Ok(resolved)
            } else {
                Err(ColdbrewError::VersionNotAvailable {
                    name: name.to_string(),
                    requested: requested.to_string(),
                    available: resolved,
                })
            }
        }
        Err(_) => Err(ColdbrewError::VersionNotAvailable {
            name: name.to_string(),
            requested: requested.to_string(),
            available: resolved,
        }),
    }
}

#[derive(Clone)]
struct BottlePlan {
    tag: String,
    file: BottleFile,
}

struct PrepareContext<'a> {
    paths: &'a Paths,
    cache: &'a Cache,
    store: &'a Store,
    ghcr: &'a GhcrClient,
}

struct PrepareResult {
    bytes_downloaded: u64,
    download_duration: Duration,
    extract_duration: Duration,
    total_duration: Duration,
    downloaded: bool,
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
}

struct DownloadResult {
    name: String,
    version: String,
    bytes_downloaded: u64,
    download_duration: Duration,
    extract_duration: Duration,
    total_duration: Duration,
    downloaded: bool,
}

struct DownloadHandle {
    handle: JoinHandle<Result<DownloadResult>>,
}

struct DownloadManager {
    handles: HashMap<String, DownloadHandle>,
    progress: Option<Arc<DownloadProgress>>,
    bytes_downloaded: u64,
    download_duration: Duration,
    extract_duration: Duration,
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
        store: Arc<Store>,
        ghcr: Arc<GhcrClient>,
        groups: Vec<DownloadGroup>,
        download_semaphore: Arc<Semaphore>,
        extract_semaphore: Arc<Semaphore>,
        paths: Paths,
        output: &Output,
    ) -> Self {
        let progress = DownloadProgress::new(groups.len(), "Downloading bottles");
        let mut handles = HashMap::new();

        for group in groups {
            let sha256 = group.sha256.clone();
            let cache = cache.clone();
            let store = store.clone();
            let ghcr = ghcr.clone();
            let download_semaphore = download_semaphore.clone();
            let extract_semaphore = extract_semaphore.clone();
            let progress = progress.clone();
            let paths = paths.clone();

            let handle = tokio::spawn(async move {
                let result = download_group(
                    &paths,
                    ghcr.as_ref(),
                    cache.as_ref(),
                    store.as_ref(),
                    download_semaphore,
                    extract_semaphore,
                    group,
                )
                .await;
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

        Self {
            handles,
            progress,
            bytes_downloaded: 0,
            download_duration: Duration::ZERO,
            extract_duration: Duration::ZERO,
        }
    }

    async fn wait_for(&mut self, sha256: &str, output: &Output) -> Result<()> {
        if let Some(handle) = self.handles.remove(sha256) {
            let result = handle
                .handle
                .await
                .map_err(|err| ColdbrewError::Other(err.to_string()))??;
            self.record_result(&result, output);
        }
        Ok(())
    }

    async fn wait_for_all(&mut self, output: &Output) -> Result<()> {
        let keys: Vec<String> = self.handles.keys().cloned().collect();
        for key in keys {
            self.wait_for(&key, output).await?;
        }
        if let Some(progress) = self.progress.as_ref() {
            progress.bar.finish_and_clear();
        }
        self.print_summary(output);
        Ok(())
    }

    fn record_result(&mut self, result: &DownloadResult, output: &Output) {
        if result.downloaded && result.bytes_downloaded > 0 {
            output.debug(&format!(
                "Downloaded {} {} ({}) in {}",
                result.name,
                result.version,
                format_bytes(result.bytes_downloaded),
                format_duration(result.download_duration.as_secs())
            ));
        }

        if result.total_duration > Duration::ZERO {
            output.debug(&format!(
                "Prepared {} {} in {}",
                result.name,
                result.version,
                format_duration(result.total_duration.as_secs())
            ));
        }

        self.bytes_downloaded += result.bytes_downloaded;
        self.download_duration += result.download_duration;
        self.extract_duration += result.extract_duration;
    }

    fn print_summary(&self, output: &Output) {
        if self.bytes_downloaded > 0 {
            output.debug(&format!(
                "Downloaded {} total in {}",
                format_bytes(self.bytes_downloaded),
                format_duration(self.download_duration.as_secs())
            ));
        }
        if self.extract_duration > Duration::ZERO {
            output.debug(&format!(
                "Store extraction time {}",
                format_duration(self.extract_duration.as_secs())
            ));
        }
    }
}

fn plan_download_groups(
    ctx: &InstallContext<'_>,
    install_order: &[String],
    formula_map: &HashMap<String, Formula>,
    root: &str,
    root_version: Option<&str>,
    force: bool,
    version_policy: VersionPolicy,
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
            match root_version {
                Some(requested) => {
                    resolve_requested_version(pkg_name, requested, formula, version_policy)?
                }
                None => formula.version_with_revision(),
            }
        } else {
            formula.version_with_revision()
        };

        if is_root && ctx.cellar.is_installed(pkg_name, &target_version) && !force {
            return Err(ColdbrewError::PackageAlreadyInstalled {
                name: pkg_name.clone(),
                version: target_version.clone(),
            });
        }

        let bottle_plan = resolve_bottle(formula, ctx.platform, pkg_name)?;

        let sha256 = bottle_plan.file.sha256.clone();
        if ctx.store.entry_exists(&sha256) {
            continue;
        }

        if groups.contains_key(&sha256) {
            continue;
        }

        groups.insert(
            sha256.clone(),
            DownloadGroup {
                sha256,
                name: pkg_name.clone(),
                version: target_version.clone(),
                tag: bottle_plan.tag.clone(),
                formula: formula.clone(),
                bottle_file: bottle_plan.file.clone(),
            },
        );
    }

    Ok(groups.into_values().collect())
}

async fn download_group(
    paths: &Paths,
    ghcr: &GhcrClient,
    cache: &Cache,
    store: &Store,
    download_semaphore: Arc<Semaphore>,
    extract_semaphore: Arc<Semaphore>,
    group: DownloadGroup,
) -> Result<DownloadResult> {
    let bottle_plan = BottlePlan {
        tag: group.tag.clone(),
        file: group.bottle_file.clone(),
    };

    let prepare_ctx = PrepareContext {
        paths,
        cache,
        store,
        ghcr,
    };

    let prepare = prepare_bottle(
        &prepare_ctx,
        &group.formula,
        &bottle_plan,
        &group.name,
        &group.version,
        download_semaphore,
        extract_semaphore,
    )
    .await?;

    Ok(DownloadResult {
        name: group.name,
        version: group.version,
        bytes_downloaded: prepare.bytes_downloaded,
        download_duration: prepare.download_duration,
        extract_duration: prepare.extract_duration,
        total_duration: prepare.total_duration,
        downloaded: prepare.downloaded,
    })
}

async fn prepare_bottle(
    ctx: &PrepareContext<'_>,
    formula: &Formula,
    bottle_plan: &BottlePlan,
    name: &str,
    version: &str,
    download_semaphore: Arc<Semaphore>,
    extract_semaphore: Arc<Semaphore>,
) -> Result<PrepareResult> {
    let start = Instant::now();
    let mut download_duration = Duration::ZERO;
    let mut extract_duration = Duration::ZERO;
    let mut bytes_downloaded = 0;
    let mut downloaded = false;

    let sha256 = &bottle_plan.file.sha256;

    for attempt in 1..=2 {
        let dest = ctx.cache.blob_path(sha256);
        if !dest.exists() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            let temp_path = ctx.cache.blob_temp_path(sha256);
            if temp_path.exists() {
                let _ = fs::remove_file(&temp_path);
            }

            let downloaded_bytes = Arc::new(AtomicU64::new(0));
            let downloaded_bytes_clone = downloaded_bytes.clone();
            let _permit = download_semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|err| {
                    ColdbrewError::Other(format!("Failed to acquire download slot: {}", err))
                })?;
            let download_start = Instant::now();
            ctx.ghcr
                .download_bottle(formula, &bottle_plan.file, &temp_path, |downloaded, _| {
                    downloaded_bytes_clone.store(downloaded, Ordering::Relaxed);
                })
                .await?;
            fs::rename(&temp_path, &dest)?;
            let reported = downloaded_bytes.load(Ordering::Relaxed);
            let size = dest.metadata().map(|meta| meta.len()).unwrap_or(0);
            let size = if reported > 0 { reported } else { size };
            ctx.cache.record_blob_metadata(
                sha256,
                Some(name),
                Some(version),
                Some(&bottle_plan.tag),
                size,
            )?;

            downloaded = true;
            download_duration += download_start.elapsed();
            bytes_downloaded = bytes_downloaded.max(size);
        } else {
            let size = dest.metadata().map(|meta| meta.len()).unwrap_or(0);
            ctx.cache.record_blob_metadata(
                sha256,
                Some(name),
                Some(version),
                Some(&bottle_plan.tag),
                size,
            )?;
        }

        if let Err(err) = verify::verify_bottle(&dest, sha256, name) {
            if attempt < 2 {
                ctx.cache.remove(sha256)?;
                continue;
            }
            return Err(err);
        }

        let extract_start = Instant::now();
        let entry = {
            let _permit = extract_semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|err| {
                    ColdbrewError::Other(format!("Failed to acquire extraction slot: {}", err))
                })?;
            ctx.store.ensure_entry(sha256, &dest)
        };
        match entry {
            Ok(entry) => {
                extract_duration += extract_start.elapsed();
                let db = Database::new(ctx.paths.clone());
                let conn = db.connect()?;
                db.upsert_store_entry(&conn, sha256, entry.size_bytes)?;
                break;
            }
            Err(err) => {
                if attempt < 2 {
                    ctx.cache.remove(sha256)?;
                    continue;
                }
                return Err(err);
            }
        }
    }

    Ok(PrepareResult {
        bytes_downloaded,
        download_duration,
        extract_duration,
        total_duration: start.elapsed(),
        downloaded,
    })
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

    if !ctx.store.entry_exists(&bottle_plan.file.sha256) {
        ctx.output.debug("Preparing bottle for store...");
        let prepare_ctx = PrepareContext {
            paths: ctx.paths,
            cache: ctx.cache,
            store: ctx.store,
            ghcr: ctx.ghcr,
        };
        let prepare = prepare_bottle(
            &prepare_ctx,
            formula,
            &bottle_plan,
            name,
            version,
            ctx.download_semaphore.clone(),
            ctx.extract_semaphore.clone(),
        )
        .await?;

        if prepare.downloaded && prepare.bytes_downloaded > 0 {
            ctx.output.debug(&format!(
                "Downloaded {} {} ({}) in {}",
                name,
                version,
                format_bytes(prepare.bytes_downloaded),
                format_duration(prepare.download_duration.as_secs())
            ));
        }

        if prepare.extract_duration > Duration::ZERO {
            ctx.output.debug(&format!(
                "Prepared store entry in {}",
                format_duration(prepare.extract_duration.as_secs())
            ));
        }
    }

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
    let store_size = ctx.store.entry_size(&bottle_plan.file.sha256)?;
    db.upsert_store_entry(&conn, &bottle_plan.file.sha256, store_size)?;

    if ctx.platform.os == Os::MacOS {
        ctx.output.debug("Relocating bottle...");
        let summary =
            match relocate::relocate_bottle(&install_path, ctx.paths, ctx.platform, ctx.output) {
                Ok(summary) => summary,
                Err(err) => {
                    cleanup_failed_install(ctx, name, version);
                    return Err(err);
                }
            };
        if summary.relocated_files > 0 {
            ctx.output.debug(&format!(
                "Relocated {} Mach-O files",
                summary.relocated_files
            ));
            ctx.output.debug("Codesigning Mach-O files...");
            let _permit = ctx
                .codesign_semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|err| {
                    ColdbrewError::Other(format!("Failed to acquire codesign slot: {}", err))
                })?;
            if let Err(err) = relocate::codesign_macho_tree(&install_path, ctx.platform, ctx.output)
            {
                cleanup_failed_install(ctx, name, version);
                return Err(err);
            }
        }
    }

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
    db.add_store_ref(&conn, &bottle_plan.file.sha256, name, version)?;

    ctx.output.debug(&format!(
        "Installed {} {} in {}",
        name,
        version,
        format_duration(install_start.elapsed().as_secs())
    ));

    Ok(installed)
}

fn cleanup_failed_install(ctx: &InstallContext<'_>, name: &str, version: &str) {
    if let Err(err) = ctx.cellar.uninstall(name, version) {
        ctx.output
            .debug(&format!("Failed to cleanup {} {}: {}", name, version, err));
    }
}
