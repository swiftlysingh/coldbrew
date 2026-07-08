import Foundation

public struct InstallRequest: Equatable, Sendable {
    public var name: String
    public var version: String
    public var bottleURL: URL
    public var sha256: String
    public var tag: String
    public var tap: String
    public var binaries: [String]
    public var runtimeDependencies: [RuntimeDependencyRecord]
    public var installedAsDependency: Bool
    public var installedFor: String?

    public init(
        name: String,
        version: String,
        bottleURL: URL,
        sha256: String,
        tag: String,
        tap: String = "homebrew/core",
        binaries: [String] = [],
        runtimeDependencies: [RuntimeDependencyRecord] = [],
        installedAsDependency: Bool = false,
        installedFor: String? = nil
    ) {
        self.name = name
        self.version = version
        self.bottleURL = bottleURL
        self.sha256 = sha256
        self.tag = tag
        self.tap = tap
        self.binaries = binaries
        self.runtimeDependencies = runtimeDependencies
        self.installedAsDependency = installedAsDependency
        self.installedFor = installedFor
    }
}

public struct InstallOptions: Equatable, Sendable {
    public var force: Bool
    public var link: Bool
    public var maxConcurrentDownloads: Int

    public init(force: Bool = false, link: Bool = true, maxConcurrentDownloads: Int = 1) {
        self.force = force
        self.link = link
        self.maxConcurrentDownloads = max(1, maxConcurrentDownloads)
    }
}

public struct InstallResult: Equatable, Sendable {
    public var packages: [InstalledPackageRecord]
    public var downloadedBytes: UInt64
}

public struct InstallManager: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    @discardableResult
    public func install(_ requests: [InstallRequest], options: InstallOptions = InstallOptions()) async throws -> InstallResult {
        try paths.createDirectories()
        let downloads = try await download(requests, limit: options.maxConcurrentDownloads)
        let cellar = Cellar(paths: paths)
        let store = Store(paths: paths)
        let db = Database(paths: paths)
        let connection = try db.connect()
        var installed: [InstalledPackageRecord] = []

        for request in requests {
            if cellar.isInstalled(name: request.name, version: request.version) {
                if options.force {
                    try cellar.uninstall(name: request.name, version: request.version)
                } else {
                    throw ColdbrewError.packageAlreadyInstalled(name: request.name, version: request.version)
                }
            }

            let download = downloads[request.key]!
            let entry = try store.ensureEntry(sha256: request.sha256, bottleArchive: download.path)
            try db.upsertStoreEntry(connection, sha256: request.sha256, sizeBytes: entry.sizeBytes)
            let installPath = try store.materialize(sha256: request.sha256, name: request.name, version: request.version)
            let relocator = BottleRelocator(paths: paths)
            _ = try relocator.relocateBottle(at: installPath)
            _ = try relocator.codesignMachOTree(at: installPath)

            var package = InstalledPackageRecord(
                name: request.name,
                version: request.version,
                tap: request.tap,
                cellarPath: installPath.path,
                runtimeDependencies: request.runtimeDependencies,
                linked: false,
                bottleTag: request.tag,
                bottleSha256: request.sha256,
                binaries: request.binaries,
                installedAsDependency: request.installedAsDependency,
                installedFor: request.installedFor
            )

            if options.link, !request.binaries.isEmpty {
                try ShimManager(paths: paths).createShims(name: request.name, version: request.version, binaries: request.binaries)
                package.linked = true
            }
            try cellar.saveMetadata(PackageMetadataRecord(package: package, receipt: InstallReceiptRecord(source: request.bottleURL.absoluteString)))
            try db.addStoreRef(connection, sha256: request.sha256, package: request.name, version: request.version)
            installed.append(package)
        }

        return InstallResult(packages: installed, downloadedBytes: downloads.values.reduce(UInt64(0)) { $0 + $1.bytesDownloaded })
    }

    private func download(_ requests: [InstallRequest], limit: Int) async throws -> [String: BottleDownloadResult] {
        var results: [String: BottleDownloadResult] = [:]
        let downloader = BottleDownloader(cache: Cache(paths: paths))
        var index = 0
        while index < requests.count {
            let chunk = Array(requests[index..<min(index + limit, requests.count)])
            let chunkResults = try await withThrowingTaskGroup(of: (String, BottleDownloadResult).self) { group in
                for request in chunk {
                    group.addTask {
                        let result = try await downloader.downloadToCache(BottleDownloadRequest(
                            url: request.bottleURL,
                            sha256: request.sha256,
                            name: request.name,
                            version: request.version,
                            tag: request.tag
                        ))
                        return (request.key, result)
                    }
                }

                var pairs: [(String, BottleDownloadResult)] = []
                for try await pair in group {
                    pairs.append(pair)
                }
                return pairs
            }
            for (key, result) in chunkResults {
                results[key] = result
            }
            index += limit
        }
        return results
    }
}

private extension InstallRequest {
    var key: String { "\(name)@\(version)" }
}
