import ArgumentParser
import ColdbrewKit
import Foundation

enum CrewCompletionShell: String, ExpressibleByArgument {
    case bash
    case elvish
    case fish
    case powershell
    case zsh
}

@main
struct Crew: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "crew",
        abstract: "Coldbrew - A Homebrew-compatible package manager",
        version: ColdbrewKit.version,
        subcommands: [
            Update.self,
            Search.self,
            Info.self,
            Install.self,
            Uninstall.self,
            Upgrade.self,
            List.self,
            Which.self,
            Pin.self,
            Unpin.self,
            Default.self,
            Dependents.self,
            Init.self,
            Lock.self,
            Tap.self,
            Space.self,
            Link.self,
            Unlink.self,
            Doctor.self,
            Completions.self,
            Exec.self,
        ]
    )

    @Flag(name: .shortAndLong, help: "Enable verbose output")
    var verbose = false

    @Flag(name: .shortAndLong, help: "Suppress non-error output")
    var quiet = false

    func run() throws {
        throw CleanExit.helpRequest(self)
    }
}

struct Update: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Update the package index from Homebrew")

    func run() throws {
        let count = try FormulaIndex(paths: Paths()).update(from: HomebrewAPI())
        print("Updated \(count) formulae")
    }
}

struct Search: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Search for packages")

    @Argument(help: "Search query")
    var query: String

    @Flag(name: .shortAndLong, help: "Show extended information")
    var extended = false

    func run() throws {
        let matches = try FormulaIndex(paths: Paths()).search(query)
        for formula in matches {
            if extended, let desc = formula.desc {
                print("\(formula.name): \(desc)")
            } else {
                print(formula.name)
            }
        }
    }
}

struct Info: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Show information about a package")

    @Argument(help: "Package name")
    var package: String

    @Option(name: .shortAndLong, help: "Output format (text, json)")
    var format = "text"

    func run() throws {
        let formula = try FormulaIndex(paths: Paths()).formula(named: package)
            ?? HomebrewAPI().fetchFormula(named: package)
        if format == "json" {
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
            print(String(decoding: try encoder.encode(formula), as: UTF8.self))
        } else {
            print("\(formula.name) \(formula.versionWithRevision)")
            if let desc = formula.desc {
                print(desc)
            }
            if let homepage = formula.homepage {
                print(homepage)
            }
        }
    }
}

struct Install: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Install packages")

    @Argument(help: "Packages to install (e.g., jq, node@22)")
    var packages: [String]

    @Flag(name: .long, help: "Install from coldbrew.lock instead of individual packages")
    var lock = false

    @Flag(name: .long, help: "Skip dependency installation")
    var skipDeps = false

    @Flag(name: .shortAndLong, help: "Force reinstall even if already installed")
    var force = false

    func validate() throws {
        if lock && !packages.isEmpty {
            throw ValidationError("--lock cannot be used with package arguments")
        }
        if packages.isEmpty && !lock {
            throw ValidationError("Missing expected argument '<packages>'")
        }
    }

    func run() async throws {
        let paths = try Paths()
        let requests = try installRequests(for: packages, paths: paths, includeDependencies: !skipDeps)
        let result = try await InstallManager(paths: paths).install(
            requests,
            options: InstallOptions(force: force, maxConcurrentDownloads: GlobalConfig.load(from: paths.configFile).settings.parallelDownloads)
        )
        for package in result.packages {
            print("Installed \(package.name) \(package.version)")
        }
    }
}

struct Uninstall: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Uninstall packages")

    @Argument(help: "Packages to uninstall")
    var packages: [String]

    @Flag(name: .shortAndLong, help: "Remove all versions")
    var all = false

    @Flag(name: .long, help: "Also remove unused dependencies")
    var withDeps = false

    func validate() throws {
        if packages.isEmpty {
            throw ValidationError("Missing expected argument '<packages>'")
        }
    }

    func run() throws {
        let manager = UninstallCleanupManager(paths: try Paths())
        for package in packages {
            let result = try manager.uninstall(package, options: UninstallOptions(all: all, withDeps: withDeps))
            for removed in result.removed {
                print("Uninstalled \(removed.name) \(removed.version)")
            }
        }
    }
}

struct Upgrade: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Upgrade installed packages")

    @Argument(help: "Packages to upgrade (all if not specified)")
    var packages: [String] = []

    @Flag(name: .shortAndLong, help: "Skip interactive selection")
    var yes = false

    func run() async throws {
        let paths = try Paths()
        let available = try installedUpgradeRequests(paths: paths, filter: packages)
        let manager = UpgradeManager(paths: paths)
        let plan = try manager.plan(available: available, filter: packages)
        if plan.upgrades.isEmpty {
            print("No upgrades available")
            return
        }
        for upgrade in plan.upgrades {
            print("\(upgrade.name) \(upgrade.currentVersion) -> \(upgrade.newVersion)")
        }
        let result = try await manager.apply(plan, available: available, yes: yes)
        for upgrade in result.upgraded {
            print("Upgraded \(upgrade.name) to \(upgrade.newVersion)")
        }
    }
}

struct List: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "List installed packages")

    @Flag(name: .shortAndLong, help: "Show only package names")
    var namesOnly = false

    @Option(name: .long, help: "Show versions for a specific package")
    var versions: String?

    func run() throws {
        let cellar = Cellar(paths: try Paths())
        if let package = versions {
            for version in try cellar.versions(name: package) {
                print(version)
            }
            return
        }
        for package in try cellar.listPackages() {
            print(namesOnly ? package.name : "\(package.name) \(package.version)")
        }
    }
}

struct Which: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Show which package provides a binary")

    @Argument(help: "Binary name")
    var binary: String

    func run() throws {
        switch try PackageOperations(paths: try Paths()).which(binary) {
        case .shim(_, let package, let path, _):
            print("\(binary): \(package) (\(path.path))")
        case .binary(_, let package, let version, let path):
            print("\(binary): \(package) \(version) (\(path.path))")
        case .notFound:
            throw ColdbrewError.pathNotFound(binary)
        }
    }
}

struct Pin: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Pin a package to prevent upgrades")

    @Argument(help: "Package to pin")
    var package: String

    func run() throws {
        try PackageOperations(paths: try Paths()).pin(package)
        print("Pinned \(package)")
    }
}

struct Unpin: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Unpin a package to allow upgrades")

    @Argument(help: "Package to unpin")
    var package: String

    func run() throws {
        let removed = try PackageOperations(paths: try Paths()).unpin(package)
        print(removed ? "Unpinned \(package)" : "\(package) was not pinned")
    }
}

struct Default: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Set or show the default version for a package")

    @Argument(help: "Package name (e.g., node@22 or just node to show current)")
    var package: String

    func run() throws {
        let operations = PackageOperations(paths: try Paths())
        if package.contains("@") {
            try operations.setDefault(package)
            print("Set default \(package)")
        } else {
            let result = try operations.defaultVersions(package)
            if let defaultVersion = result.defaultVersion {
                print(defaultVersion)
            } else {
                for version in result.versions {
                    print(version)
                }
            }
        }
    }
}

struct Dependents: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Show packages that depend on a package")

    @Argument(help: "Package name")
    var package: String

    func run() throws {
        for dependent in try UninstallCleanupManager(paths: try Paths()).dependents(of: package) {
            print(dependent)
        }
    }
}

struct Init: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Initialize a new coldbrew.toml in the current directory")

    @Flag(name: .shortAndLong, help: "Force overwrite if file exists")
    var force = false

    func run() throws {
        let path = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appendingPathComponent("coldbrew.toml")
        if FileManager.default.fileExists(atPath: path.path), !force {
            throw ColdbrewError.configError("coldbrew.toml already exists. Use --force to overwrite")
        }
        try ProjectConfig().save(to: path)
        print("Created coldbrew.toml")
    }
}

struct Lock: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Generate a lockfile from coldbrew.toml")

    func run() throws {
        guard let projectFile = findProjectFile(startDirectory: URL(fileURLWithPath: FileManager.default.currentDirectoryPath)) else {
            throw ColdbrewError.projectNotFound
        }
        let config = try ProjectConfig.load(from: projectFile)
        let index = FormulaIndex(paths: try Paths())
        let locked = try config.allPackages().reduce(into: [String: LockedPackage]()) { result, pair in
            let formula = try index.formula(named: pair.key)
            result[pair.key] = LockedPackage(
                version: formula?.versionWithRevision ?? pair.value,
                sha256: currentBottle(formula)?.file.sha256,
                bottleTag: currentBottle(formula)?.tag,
                tap: formula?.tap ?? "homebrew/core",
                dependencies: formula?.dependencies ?? [],
                dev: config.devPackages[pair.key] != nil
            )
        }
        try Lockfile(packages: locked, configHash: Lockfile.hash(config.tomlForHash()))
            .save(to: lockfilePath(projectFile: projectFile))
        print("Wrote coldbrew.lock")
    }
}

struct Tap: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Add or remove taps (third-party repositories)")

    @Argument(help: "Tap to add (user/repo format)")
    var tap: String?

    @Flag(name: .shortAndLong, help: "Remove a tap instead of adding")
    var remove = false

    func run() throws {
        let manager = TapManager(paths: try Paths())
        if let tap {
            if remove {
                try manager.remove(tap)
                print("Removed tap '\(tap)'")
            } else {
                let added = try manager.add(tap)
                print("Added tap '\(added.fullName)'")
            }
        } else {
            let taps = try manager.list()
            if taps.isEmpty {
                print("No taps installed")
            } else {
                for tap in taps {
                    print(tap.fullName)
                }
            }
        }
    }
}

struct Space: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Disk usage and cleanup commands",
        subcommands: [Show.self, Clean.self]
    )

    func run() throws {
        throw CleanExit.helpRequest(self)
    }

    struct Show: ParsableCommand {
        static let configuration = CommandConfiguration(abstract: "Show disk usage and cleanup candidates")

        @Flag(name: .shortAndLong, help: "Show itemized details")
        var details = false

        func run() throws {
            let summary = try UninstallCleanupManager(paths: try Paths()).spaceSummary()
            for category in summary.categories {
                print("\(category.title): \(formatBytes(category.totalSize))")
                if details {
                    for item in category.items {
                        print("  \(item.label) \(formatBytes(item.size))")
                    }
                }
            }
            print("Total: \(formatBytes(summary.totalSize))")
        }
    }

    struct Clean: ParsableCommand {
        static let configuration = CommandConfiguration(abstract: "Cleanup old versions, cache, and other unused data")

        @Flag(name: .shortAndLong, help: "Clean everything without prompts")
        var all = false

        @Flag(name: .shortAndLong, help: "Dry run - show what would be removed")
        var dryRun = false

        func run() throws {
            let result = try UninstallCleanupManager(paths: try Paths()).clean(
                kinds: Set(CleanupKind.allCases),
                dryRun: dryRun || !all
            )
            print("\(dryRun || !all ? "Would remove" : "Removed") \(result.removed) items, \(formatBytes(result.freed))")
        }
    }
}

struct Link: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Force-link a keg-only package")

    @Argument(help: "Package to link")
    var package: String

    @Flag(name: .shortAndLong, help: "Force overwrite existing files")
    var force = false

    func run() throws {
        let result = try PackageOperations(paths: try Paths()).link(package, force: force)
        print("Linked \(result.package) \(result.version)")
    }
}

struct Unlink: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Remove links for a package")

    @Argument(help: "Package to unlink")
    var package: String

    func run() throws {
        let result = try PackageOperations(paths: try Paths()).unlink(package)
        print("Unlinked \(result.package) \(result.version)")
    }
}

struct Doctor: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Check system for potential problems")

    func run() throws {
        let report = DoctorChecker(paths: try Paths()).run()
        if report.isHealthy {
            print("Your Coldbrew installation looks good!")
            return
        }
        for issue in report.issues {
            print("issue: \(issue)")
        }
        for warning in report.warnings {
            print("warning: \(warning)")
        }
    }
}

struct Completions: ParsableCommand {
    static let configuration = CommandConfiguration(abstract: "Generate shell completions")

    @Argument(help: "Shell to generate completions for")
    var shell: CrewCompletionShell

    func run() throws {
        guard let completionShell = ArgumentParser.CompletionShell(rawValue: shell.rawValue) else {
            throw ValidationError("Unsupported completion shell '\(shell.rawValue)'")
        }
        print(Crew.completionScript(for: completionShell))
    }
}

struct Exec: ParsableCommand {
    static let configuration = CommandConfiguration(
        abstract: "Execute a binary from a package (internal use)",
        shouldDisplay: false
    )

    @Argument(help: "Package name")
    var package: String

    @Argument(help: "Binary name")
    var binary: String

    @Argument(parsing: .captureForPassthrough, help: "Arguments to pass to the binary")
    var args: [String] = []

    func run() throws {
        let paths = try Paths()
        let config = try GlobalConfig.load(from: paths.configFile)
        let projectVersions = try findProjectFile(startDirectory: URL(fileURLWithPath: FileManager.default.currentDirectoryPath))
            .map { try ProjectConfig.load(from: $0).allPackages() }
        let binaryURL = try ShimManager(paths: paths).resolveBinary(
            package: package,
            binary: binary,
            defaults: config.defaults,
            projectVersions: projectVersions
        )
        let process = Process()
        process.executableURL = binaryURL
        var forwardedArgs = args
        if forwardedArgs.first == "--" {
            forwardedArgs.removeFirst()
        }
        process.arguments = forwardedArgs
        try process.run()
        process.waitUntilExit()
        throw ExitCode(process.terminationStatus)
    }
}

private func installRequests(for packages: [String], paths: Paths, includeDependencies: Bool) throws -> [InstallRequest] {
    let index = FormulaIndex(paths: paths)
    var requests: [InstallRequest] = []
    var seen: Set<String> = []

    func add(_ name: String, installedFor: String?) throws {
        guard !seen.contains(name) else { return }
        guard let formula = try index.formula(named: name) else {
            throw ColdbrewError.packageNotFound(name)
        }
        if includeDependencies {
            for dependency in formula.dependencies {
                try add(dependency, installedFor: name)
            }
        }
        guard let bottle = currentBottle(formula) else {
            throw ColdbrewError.noBottleAvailable(package: name, platform: currentPlatform().bottleTags.joined(separator: ","))
        }
        requests.append(InstallRequest(
            name: formula.name,
            version: formula.versionWithRevision,
            bottleURL: URL(string: bottle.file.url) ?? URL(fileURLWithPath: bottle.file.url),
            sha256: bottle.file.sha256,
            tag: bottle.tag,
            tap: formula.tap,
            binaries: [binaryName(for: formula.name)],
            runtimeDependencies: formula.dependencies.map {
                RuntimeDependencyRecord(name: $0, version: "", path: paths.cellarDir.appendingPathComponent($0).path)
            },
            installedAsDependency: installedFor != nil,
            installedFor: installedFor
        ))
        seen.insert(name)
    }

    for package in packages {
        try add(package, installedFor: nil)
    }
    return requests
}

private func installedUpgradeRequests(paths: Paths, filter: [String]) throws -> [InstallRequest] {
    let installedNames = try Set(Cellar(paths: paths).listPackages().map(\.name))
    let names = filter.isEmpty ? Array(installedNames) : filter
    return try installRequests(for: names, paths: paths, includeDependencies: false)
}

private func currentBottle(_ formula: Formula?) -> (tag: String, file: BottleFile)? {
    formula?.bottle.stable?.bestForPlatform(tags: currentPlatform().bottleTags)
}

private func currentPlatform() -> Platform {
    #if os(Linux)
    let os = Platform.OS.linux
    #else
    let os = Platform.OS.macOS
    #endif

    #if arch(x86_64)
    let arch = Platform.Architecture.x86_64
    #else
    let arch = Platform.Architecture.arm64
    #endif

    return Platform(os: os, architecture: arch)
}

private func binaryName(for package: String) -> String {
    package.split(separator: "@", maxSplits: 1).first.map(String.init) ?? package
}
