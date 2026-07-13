import Foundation

public struct Paths: Equatable, Sendable {
    public let root: URL

    public init() throws {
        if let home = ProcessInfo.processInfo.environment["COLDBREW_HOME"], !home.isEmpty {
            self.root = URL(fileURLWithPath: home, isDirectory: true)
        } else {
            self.root = FileManager.default.homeDirectoryForCurrentUser
                .appendingPathComponent(".coldbrew", isDirectory: true)
        }
    }

    public init(root: URL) {
        self.root = root
    }

    public func createDirectories() throws {
        for directory in [
            root,
            binDir,
            cellarDir,
            cacheDir,
            cacheBlobsDir,
            tapsDir,
            indexDir,
            logsDir,
            dbDir,
            storeDir,
            locksDir,
        ] {
            do {
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            } catch {
                throw ColdbrewError.directoryCreationFailed(directory.path)
            }
        }
    }

    public var binDir: URL { root.appendingPathComponent("bin", isDirectory: true) }
    public var cellarDir: URL { root.appendingPathComponent("cellar", isDirectory: true) }
    public var cacheDir: URL { root.appendingPathComponent("cache", isDirectory: true) }
    public var downloadsDir: URL { cacheDir.appendingPathComponent("downloads", isDirectory: true) }
    public var cacheBlobsDir: URL { cacheDir.appendingPathComponent("blobs", isDirectory: true) }
    public var tapsDir: URL { root.appendingPathComponent("taps", isDirectory: true) }
    public var indexDir: URL { root.appendingPathComponent("index", isDirectory: true) }
    public var logsDir: URL { root.appendingPathComponent("logs", isDirectory: true) }
    public var dbDir: URL { root.appendingPathComponent("db", isDirectory: true) }
    public var storeDir: URL { root.appendingPathComponent("store", isDirectory: true) }
    public var locksDir: URL { root.appendingPathComponent("locks", isDirectory: true) }
    public var configFile: URL { root.appendingPathComponent("config.toml") }
    public var formulaIndex: URL { indexDir.appendingPathComponent("formula.json") }
    public var dbFile: URL { dbDir.appendingPathComponent("coldbrew.sqlite3") }
    public var defaultsFile: URL { root.appendingPathComponent("defaults.json") }
    public var pinsFile: URL { root.appendingPathComponent("pins.json") }
    public var shimsLock: URL { locksDir.appendingPathComponent("shims.lock") }

    public func cellarPackage(_ name: String, version: String) -> URL {
        cellarDir.appendingPathComponent(name, isDirectory: true)
            .appendingPathComponent(version, isDirectory: true)
    }

    public func tapDir(user: String, repo: String) -> URL {
        tapsDir.appendingPathComponent(user, isDirectory: true)
            .appendingPathComponent(repo, isDirectory: true)
    }

    public func cacheBottle(name: String, version: String, tag: String) -> URL {
        downloadsDir.appendingPathComponent("\(name)-\(version).\(tag).bottle.tar.gz")
    }

    public func cacheBlob(sha256: String) -> URL {
        cacheBlobsDir.appendingPathComponent("\(sha256).bottle.tar.gz")
    }

    public func cacheBlobTemp(sha256: String) -> URL {
        cacheBlobsDir.appendingPathComponent("\(sha256).part")
    }

    public func storeEntry(sha256: String) -> URL {
        storeDir.appendingPathComponent(sha256, isDirectory: true)
    }

    public func storeLock(sha256: String) -> URL {
        locksDir.appendingPathComponent("\(sha256).lock")
    }

    public func shim(_ name: String) -> URL {
        binDir.appendingPathComponent(name)
    }

    public func packageMetadata(name: String, version: String) -> URL {
        cellarPackage(name, version: version).appendingPathComponent(".coldbrew.json")
    }

    public func isColdbrewPath(_ path: URL) -> Bool {
        path.path == root.path || path.path.hasPrefix(root.path + "/")
    }
}

public func findProjectFile(startDirectory: URL) -> URL? {
    var current = startDirectory

    while true {
        let projectFile = current.appendingPathComponent("coldbrew.toml")
        if FileManager.default.fileExists(atPath: projectFile.path) {
            return projectFile
        }

        let parent = current.deletingLastPathComponent()
        if parent.path == current.path {
            return nil
        }
        current = parent
    }
}

public func lockfilePath(projectFile: URL) -> URL {
    projectFile.deletingLastPathComponent().appendingPathComponent("coldbrew.lock")
}
