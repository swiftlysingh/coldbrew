import Foundation

public struct CachedBottle: Equatable, Sendable {
    public var sha256: String
    public var name: String?
    public var version: String?
    public var tag: String?
    public var path: URL
    public var size: UInt64
    public var modified: Date

    public var label: String {
        if let name, let version, let tag {
            return "\(name) \(version) (\(tag))"
        }
        return "blob \(String(sha256.prefix(12)))"
    }
}

public struct CleanResult: Equatable, Sendable {
    public var removed: Int
    public var freed: UInt64

    public init(removed: Int = 0, freed: UInt64 = 0) {
        self.removed = removed
        self.freed = freed
    }

    public var freedHuman: String {
        formatBytes(freed)
    }
}

public struct Cache: Sendable {
    private static let metadataLock = NSLock()
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    public func initialize() throws {
        try FileManager.default.createDirectory(at: paths.cacheBlobsDir, withIntermediateDirectories: true)
    }

    public func blobPath(sha256: String) -> URL {
        paths.cacheBlob(sha256: sha256)
    }

    public func blobTempPath(sha256: String) -> URL {
        paths.cacheBlobTemp(sha256: sha256)
    }

    public func isCached(sha256: String) -> Bool {
        FileManager.default.fileExists(atPath: blobPath(sha256: sha256).path)
    }

    public func cachedPath(sha256: String) -> URL? {
        let path = blobPath(sha256: sha256)
        return FileManager.default.fileExists(atPath: path.path) ? path : nil
    }

    @discardableResult
    public func storeBlob(sha256: String, data: Data) throws -> URL {
        try initialize()
        let path = blobPath(sha256: sha256)
        try data.write(to: path)
        return path
    }

    @discardableResult
    public func moveToCache(from source: URL, sha256: String) throws -> URL {
        try initialize()
        let destination = blobPath(sha256: sha256)
        do {
            try FileManager.default.moveItem(at: source, to: destination)
        } catch {
            guard FileManager.default.fileExists(atPath: destination.path) else { throw error }
            try? FileManager.default.removeItem(at: source)
        }
        return destination
    }

    public func recordBlobMetadata(
        sha256: String,
        name: String?,
        version: String?,
        tag: String?,
        sizeBytes: UInt64
    ) throws {
        Self.metadataLock.lock()
        defer { Self.metadataLock.unlock() }
        let db = Database(paths: paths)
        let connection = try db.connect()
        try db.upsertBlobCache(connection, sha256: sha256, name: name, version: version, tag: tag, sizeBytes: sizeBytes)
    }

    public func remove(sha256: String) throws {
        let path = blobPath(sha256: sha256)
        if FileManager.default.fileExists(atPath: path.path) {
            try FileManager.default.removeItem(at: path)
        }

        let db = Database(paths: paths)
        let connection = try db.connect()
        try db.deleteBlobCache(connection, sha256: sha256)
    }

    public func list() throws -> [CachedBottle] {
        guard FileManager.default.fileExists(atPath: paths.cacheBlobsDir.path) else {
            return []
        }

        let db = Database(paths: paths)
        let connection = try db.connect()
        let entries = try db.listBlobCache(connection)
        let materialized = try materialize(entries)
        return materialized.isEmpty ? try scanBlobDirectory() : materialized
    }

    public func clean(maxAge: TimeInterval? = nil) throws -> CleanResult {
        guard FileManager.default.fileExists(atPath: paths.cacheBlobsDir.path) else {
            return CleanResult()
        }

        let db = Database(paths: paths)
        let connection = try db.connect()
        let now = Date()
        var result = CleanResult()

        let files = try FileManager.default.contentsOfDirectory(at: paths.cacheBlobsDir, includingPropertiesForKeys: [.contentModificationDateKey, .fileSizeKey])
        for file in files where file.lastPathComponent.hasSuffix(".bottle.tar.gz") {
            let values = try file.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey])
            let modified = values.contentModificationDate ?? .distantPast
            let shouldRemove = maxAge.map { now.timeIntervalSince(modified) > $0 } ?? true
            guard shouldRemove else { continue }

            let size = UInt64(values.fileSize ?? 0)
            let sha = file.lastPathComponent.replacingOccurrences(of: ".bottle.tar.gz", with: "")
            try db.deleteBlobCache(connection, sha256: sha)
            try FileManager.default.removeItem(at: file)
            result.removed += 1
            result.freed += size
        }
        return result
    }

    public func totalSize() throws -> UInt64 {
        guard FileManager.default.fileExists(atPath: paths.cacheBlobsDir.path) else {
            return 0
        }

        let files = try FileManager.default.contentsOfDirectory(at: paths.cacheBlobsDir, includingPropertiesForKeys: [.fileSizeKey])
        return try files.reduce(UInt64(0)) { total, file in
            let values = try file.resourceValues(forKeys: [.fileSizeKey])
            return total + UInt64(values.fileSize ?? 0)
        }
    }

    private func materialize(_ entries: [BlobCacheEntry]) throws -> [CachedBottle] {
        try entries.compactMap { entry in
            let path = blobPath(sha256: entry.sha256)
            guard FileManager.default.fileExists(atPath: path.path) else { return nil }
            let values = try path.resourceValues(forKeys: [.contentModificationDateKey])
            return CachedBottle(
                sha256: entry.sha256,
                name: entry.name,
                version: entry.version,
                tag: entry.tag,
                path: path,
                size: entry.sizeBytes,
                modified: values.contentModificationDate ?? .distantPast
            )
        }.sorted { $0.label < $1.label }
    }

    private func scanBlobDirectory() throws -> [CachedBottle] {
        let files = try FileManager.default.contentsOfDirectory(at: paths.cacheBlobsDir, includingPropertiesForKeys: [.contentModificationDateKey, .fileSizeKey])
        return try files.compactMap { file in
            guard file.lastPathComponent.hasSuffix(".bottle.tar.gz") else { return nil }
            let values = try file.resourceValues(forKeys: [.contentModificationDateKey, .fileSizeKey])
            let sha = file.lastPathComponent.replacingOccurrences(of: ".bottle.tar.gz", with: "")
            return CachedBottle(
                sha256: sha,
                name: nil,
                version: nil,
                tag: nil,
                path: file,
                size: UInt64(values.fileSize ?? 0),
                modified: values.contentModificationDate ?? .distantPast
            )
        }.sorted { $0.label < $1.label }
    }
}

public func formatBytes(_ bytes: UInt64) -> String {
    let kb: UInt64 = 1024
    let mb = kb * 1024
    let gb = mb * 1024

    if bytes >= gb {
        return String(format: "%.2f GB", Double(bytes) / Double(gb))
    }
    if bytes >= mb {
        return String(format: "%.2f MB", Double(bytes) / Double(mb))
    }
    if bytes >= kb {
        return String(format: "%.2f KB", Double(bytes) / Double(kb))
    }
    return "\(bytes) B"
}
