import Foundation

public struct UninstallOptions: Equatable, Sendable {
    public var all: Bool
    public var withDeps: Bool

    public init(all: Bool = false, withDeps: Bool = false) {
        self.all = all
        self.withDeps = withDeps
    }
}

public struct RemovedPackage: Equatable, Sendable {
    public var name: String
    public var version: String
}

public struct UninstallResult: Equatable, Sendable {
    public var removed: [RemovedPackage]
}

public enum CleanupKind: String, CaseIterable, Sendable {
    case cacheDownloads
    case orphanedStore
}

public struct CleanupItem: Equatable, Sendable {
    public var kind: CleanupKind
    public var label: String
    public var path: URL
    public var size: UInt64
    public var name: String?
    public var version: String?
}

public struct CleanupCategory: Equatable, Sendable {
    public var kind: CleanupKind
    public var title: String
    public var items: [CleanupItem]

    public var totalSize: UInt64 {
        items.reduce(0) { $0 + $1.size }
    }
}

public struct CleanupResult: Equatable, Sendable {
    public var removed: Int = 0
    public var freed: UInt64 = 0
}

public struct SpaceSummary: Equatable, Sendable {
    public var categories: [CleanupCategory]

    public var totalSize: UInt64 {
        categories.reduce(0) { $0 + $1.totalSize }
    }
}

public struct UninstallCleanupManager: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    @discardableResult
    public func uninstall(_ packageSpec: String, options: UninstallOptions = UninstallOptions()) throws -> UninstallResult {
        let (name, requestedVersion) = parsePackageSpec(packageSpec)
        let cellar = Cellar(paths: paths)
        let versions = try cellar.versions(name: name)
        guard !versions.isEmpty else {
            throw ColdbrewError.packageNotInstalled(name: name, version: requestedVersion ?? "any")
        }

        let versionsToRemove: [String]
        if options.all {
            versionsToRemove = versions
        } else if let requestedVersion {
            guard versions.contains(requestedVersion) else {
                throw ColdbrewError.packageNotInstalled(name: name, version: requestedVersion)
            }
            versionsToRemove = [requestedVersion]
        } else {
            versionsToRemove = [versions.last!]
        }

        var removed: [RemovedPackage] = []
        for version in versionsToRemove {
            let package = try? cellar.package(name: name, version: version)
            let binaries = try cellar.binaries(name: name, version: version)
            try cellar.uninstall(name: name, version: version)
            if let sha = package?.bottleSha256 {
                let db = Database(paths: paths)
                try db.removeStoreRef(db.connect(), sha256: sha, package: name, version: version)
            }
            removed.append(RemovedPackage(name: name, version: version))

            let remaining = try cellar.versions(name: name)
            if remaining.isEmpty {
                try ShimManager(paths: paths).removeShims(binaries: binaries)
                var config = try SimpleGlobalConfig.load(paths: paths)
                config.defaults.removeValue(forKey: name)
                config.pins.removeValue(forKey: name)
                try config.save(paths: paths)
            }
        }

        if options.withDeps {
            while let dep = try orphanDependencies().first {
                removed += try uninstall("\(dep.name)@\(dep.version)").removed
            }
        }
        return UninstallResult(removed: removed)
    }

    public func dependents(of package: String) throws -> [String] {
        try Cellar(paths: paths).listPackages().compactMap { installed in
            installed.runtimeDependencies.contains { $0.name == package } ? installed.name : nil
        }.sorted()
    }

    public func collectCleanup() throws -> [CleanupCategory] {
        [
            CleanupCategory(kind: .cacheDownloads, title: "Cache downloads", items: try cacheItems()),
            CleanupCategory(kind: .orphanedStore, title: "Orphaned store", items: try orphanStoreItems()),
        ]
    }

    public func spaceSummary() throws -> SpaceSummary {
        SpaceSummary(categories: try collectCleanup())
    }

    public func clean(kinds: Set<CleanupKind>, dryRun: Bool = false) throws -> CleanupResult {
        var result = CleanupResult()
        for category in try collectCleanup() where kinds.contains(category.kind) {
            for item in category.items {
                if !dryRun {
                    switch item.kind {
                    case .cacheDownloads:
                        if let name = item.name {
                            try Cache(paths: paths).remove(sha256: name)
                        }
                    case .orphanedStore:
                        if let name = item.name {
                            try Store(paths: paths).removeEntry(sha256: name)
                            let db = Database(paths: paths)
                            try db.deleteStoreEntry(db.connect(), sha256: name)
                        }
                    }
                }
                result.removed += 1
                result.freed += item.size
            }
        }
        return result
    }

    private func cacheItems() throws -> [CleanupItem] {
        try Cache(paths: paths).list().map {
            CleanupItem(kind: .cacheDownloads, label: $0.label, path: $0.path, size: $0.size, name: $0.sha256, version: nil)
        }
    }

    private func orphanStoreItems() throws -> [CleanupItem] {
        let db = Database(paths: paths)
        let connection = try db.connect()
        return try db.listOrphanedStoreEntries(connection).map {
            CleanupItem(
                kind: .orphanedStore,
                label: String($0.sha256.prefix(12)),
                path: paths.storeEntry(sha256: $0.sha256),
                size: $0.sizeBytes,
                name: $0.sha256,
                version: nil
            )
        }
    }

    private func orphanDependencies() throws -> [InstalledPackageRecord] {
        let installed = try Cellar(paths: paths).listPackages()
        let required = Set(installed.flatMap { $0.runtimeDependencies.map(\.name) })
        return installed.filter { $0.installedAsDependency && !required.contains($0.name) }
    }
}
