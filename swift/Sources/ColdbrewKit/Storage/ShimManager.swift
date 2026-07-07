import Foundation

public struct ShimInfo: Equatable, Sendable {
    public var name: String
    public var package: String
    public var path: URL
}

public struct ShimManager: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    @discardableResult
    public func createShims(name: String, version: String, binaries: [String]) throws -> [URL] {
        let lock = try FileLock(path: paths.shimsLock)
        _ = lock
        try FileManager.default.createDirectory(at: paths.binDir, withIntermediateDirectories: true)

        var created: [URL] = []
        for binary in binaries {
            let shimURL = paths.shim(binary)
            try createShim(at: shimURL, package: name, version: version, binary: binary)
            created.append(shimURL)
        }
        return created
    }

    public func removeShims(binaries: [String]) throws {
        let lock = try FileLock(path: paths.shimsLock)
        _ = lock
        for binary in binaries {
            let shimURL = paths.shim(binary)
            if FileManager.default.fileExists(atPath: shimURL.path), try isColdbrewShim(shimURL) {
                try FileManager.default.removeItem(at: shimURL)
            }
        }
    }

    public func listShims() throws -> [ShimInfo] {
        guard FileManager.default.fileExists(atPath: paths.binDir.path) else {
            return []
        }

        let entries = try FileManager.default.contentsOfDirectory(at: paths.binDir, includingPropertiesForKeys: [.isRegularFileKey])
        var shims: [ShimInfo] = []
        for entry in entries where try entry.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true {
            guard try isColdbrewShim(entry) else { continue }
            let content = try String(contentsOf: entry)
            guard let package = parsePackage(fromShim: content) else { continue }
            shims.append(ShimInfo(name: entry.lastPathComponent, package: package, path: entry))
        }
        return shims.sorted { $0.name < $1.name }
    }

    public func hasShim(binary: String) -> Bool {
        FileManager.default.fileExists(atPath: paths.shim(binary).path)
    }

    public func realBinaryPath(name: String, version: String, binary: String) -> URL {
        paths.cellarPackage(name, version: version)
            .appendingPathComponent("bin", isDirectory: true)
            .appendingPathComponent(binary)
    }

    public func resolveBinary(
        package: String,
        binary: String,
        defaults: [String: String],
        projectVersions: [String: String]? = nil
    ) throws -> URL {
        guard let version = projectVersions?[package] ?? defaults[package] else {
            throw ColdbrewError.noDefaultVersion(package)
        }

        let binaryURL = realBinaryPath(name: package, version: version, binary: binary)
        guard FileManager.default.fileExists(atPath: binaryURL.path) else {
            throw ColdbrewError.packageNotInstalled(name: package, version: version)
        }
        return binaryURL
    }

    private func createShim(at shimURL: URL, package: String, version _: String, binary: String) throws {
        let content = """
        #!/bin/sh
        # Coldbrew shim for \(package)/\(binary)
        # This shim resolves the correct version and executes the real binary

        exec crew exec \(package) \(binary) "$@"
        """
        try content.write(to: shimURL, atomically: true, encoding: .utf8)
        try setExecutable(shimURL)
    }

    private func isColdbrewShim(_ url: URL) throws -> Bool {
        try String(contentsOf: url).contains("# Coldbrew shim")
    }

    private func parsePackage(fromShim content: String) -> String? {
        for line in content.split(separator: "\n") {
            guard line.hasPrefix("# Coldbrew shim for ") else { continue }
            return line.dropFirst("# Coldbrew shim for ".count).split(separator: "/").first.map(String.init)
        }
        return nil
    }
}
