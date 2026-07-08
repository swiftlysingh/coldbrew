import Foundation

public struct StoreEntry: Equatable, Sendable {
    public var path: URL
    public var sizeBytes: UInt64
    public var created: Bool
}

public struct Store: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    public func entryExists(sha256: String) -> Bool {
        FileManager.default.fileExists(atPath: paths.storeEntry(sha256: sha256).path)
    }

    public func entrySize(sha256: String) throws -> UInt64 {
        try directorySize(paths.storeEntry(sha256: sha256))
    }

    public func entryPath(sha256: String) -> URL {
        paths.storeEntry(sha256: sha256)
    }

    @discardableResult
    public func ensureEntry(sha256: String, sourceDirectory: URL? = nil) throws -> StoreEntry {
        let entry = paths.storeEntry(sha256: sha256)
        if FileManager.default.fileExists(atPath: entry.path) {
            return StoreEntry(path: entry, sizeBytes: try directorySize(entry), created: false)
        }

        try FileManager.default.createDirectory(at: entry.deletingLastPathComponent(), withIntermediateDirectories: true)
        let lock = try FileLock(path: paths.storeLock(sha256: sha256))
        _ = lock
        if FileManager.default.fileExists(atPath: entry.path) {
            return StoreEntry(path: entry, sizeBytes: try directorySize(entry), created: false)
        }

        try FileManager.default.createDirectory(at: entry, withIntermediateDirectories: true)
        if let sourceDirectory {
            do {
                try copyTree(from: sourceDirectory, to: entry)
            } catch {
                try? FileManager.default.removeItem(at: entry)
                throw error
            }
        }

        return StoreEntry(path: entry, sizeBytes: try directorySize(entry), created: true)
    }

    public func removeEntry(sha256: String) throws {
        let entry = paths.storeEntry(sha256: sha256)
        if FileManager.default.fileExists(atPath: entry.path) {
            try FileManager.default.removeItem(at: entry)
        }
        let lock = paths.storeLock(sha256: sha256)
        if FileManager.default.fileExists(atPath: lock.path) {
            try? FileManager.default.removeItem(at: lock)
        }
    }

    @discardableResult
    public func materialize(sha256: String, name: String, version: String) throws -> URL {
        let entry = paths.storeEntry(sha256: sha256)
        guard FileManager.default.fileExists(atPath: entry.path) else {
            throw ColdbrewError.pathNotFound(entry.path)
        }

        let target = paths.cellarPackage(name, version: version)
        guard !FileManager.default.fileExists(atPath: target.path) else {
            throw ColdbrewError.packageAlreadyInstalled(name: name, version: version)
        }

        try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
        try copyTree(from: entry, to: target)
        return target
    }
}
