import Foundation

public struct LinkResult: Equatable, Sendable {
    public var package: String
    public var version: String
    public var binaries: [String]
}

public struct DefaultVersionsResult: Equatable, Sendable {
    public var package: String
    public var versions: [String]
    public var defaultVersion: String?
}

public enum WhichResult: Equatable, Sendable {
    case shim(binary: String, package: String, path: URL, versions: [String])
    case binary(binary: String, package: String, version: String, path: URL)
    case notFound(binary: String)
}

public struct PackageOperations: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    @discardableResult
    public func link(_ packageSpec: String, force: Bool = false) throws -> LinkResult {
        let (name, requestedVersion) = parsePackageSpec(packageSpec)
        let cellar = Cellar(paths: paths)
        let shimManager = ShimManager(paths: paths)
        let version = try installedVersion(name: name, requestedVersion: requestedVersion, cellar: cellar)
        let binaries = try cellar.binaries(name: name, version: version)
        guard !binaries.isEmpty else {
            return LinkResult(package: name, version: version, binaries: [])
        }

        if !force, binaries.contains(where: { shimManager.hasShim(binary: $0) }) {
            throw ColdbrewError.other("Shim already exists. Use --force to overwrite")
        }
        try shimManager.createShims(name: name, version: version, binaries: binaries)
        return LinkResult(package: name, version: version, binaries: binaries)
    }

    @discardableResult
    public func unlink(_ packageSpec: String) throws -> LinkResult {
        let (name, requestedVersion) = parsePackageSpec(packageSpec)
        let cellar = Cellar(paths: paths)
        let version = try installedVersion(name: name, requestedVersion: requestedVersion, cellar: cellar)
        let binaries = try cellar.binaries(name: name, version: version)
        try ShimManager(paths: paths).removeShims(binaries: binaries)
        return LinkResult(package: name, version: version, binaries: binaries)
    }

    public func pin(_ packageSpec: String) throws {
        let (name, requestedVersion) = parsePackageSpec(packageSpec)
        let version = try installedVersion(name: name, requestedVersion: requestedVersion, cellar: Cellar(paths: paths))
        var config = try SimpleGlobalConfig.load(paths: paths)
        config.pins[name] = version
        try config.save(paths: paths)
    }

    public func unpin(_ packageSpec: String) throws -> Bool {
        let (name, _) = parsePackageSpec(packageSpec)
        var config = try SimpleGlobalConfig.load(paths: paths)
        let removed = config.pins.removeValue(forKey: name) != nil
        try config.save(paths: paths)
        return removed
    }

    public func setDefault(_ packageSpec: String) throws {
        let (name, requestedVersion) = parsePackageSpec(packageSpec)
        guard let requestedVersion else {
            throw ColdbrewError.invalidVersion(packageSpec)
        }
        _ = try installedVersion(name: name, requestedVersion: requestedVersion, cellar: Cellar(paths: paths))
        var config = try SimpleGlobalConfig.load(paths: paths)
        config.defaults[name] = requestedVersion
        try config.save(paths: paths)
    }

    public func defaultVersions(_ packageSpec: String) throws -> DefaultVersionsResult {
        let (name, _) = parsePackageSpec(packageSpec)
        let versions = try Cellar(paths: paths).versions(name: name)
        guard !versions.isEmpty else {
            throw ColdbrewError.packageNotInstalled(name: name, version: "any")
        }
        return DefaultVersionsResult(
            package: name,
            versions: versions,
            defaultVersion: try SimpleGlobalConfig.load(paths: paths).defaults[name]
        )
    }

    public func which(_ binary: String) throws -> WhichResult {
        let cellar = Cellar(paths: paths)
        let shimManager = ShimManager(paths: paths)
        if shimManager.hasShim(binary: binary),
           let shim = try shimManager.listShims().first(where: { $0.name == binary }) {
            return .shim(binary: binary, package: shim.package, path: shim.path, versions: try cellar.versions(name: shim.package))
        }

        for package in try cellar.listPackages() {
            let binaries = try cellar.binaries(name: package.name, version: package.version)
            if binaries.contains(binary) {
                return .binary(
                    binary: binary,
                    package: package.name,
                    version: package.version,
                    path: URL(fileURLWithPath: package.cellarPath).appendingPathComponent("bin/\(binary)")
                )
            }
        }
        return .notFound(binary: binary)
    }

    private func installedVersion(name: String, requestedVersion: String?, cellar: Cellar) throws -> String {
        let versions = try cellar.versions(name: name)
        guard !versions.isEmpty else {
            throw ColdbrewError.packageNotInstalled(name: name, version: requestedVersion ?? "any")
        }
        let version = requestedVersion ?? versions.last!
        guard versions.contains(version) else {
            throw ColdbrewError.packageNotInstalled(name: name, version: version)
        }
        return version
    }
}

struct SimpleGlobalConfig: Equatable {
    var defaults: [String: String] = [:]
    var pins: [String: String] = [:]

    static func load(paths: Paths) throws -> Self {
        guard FileManager.default.fileExists(atPath: paths.configFile.path) else {
            return Self()
        }
        let text = try String(contentsOf: paths.configFile)
        var config = Self()
        var section: WritableKeyPath<Self, [String: String]>?
        for rawLine in text.split(separator: "\n", omittingEmptySubsequences: false) {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            if line == "[defaults]" {
                section = \.defaults
                continue
            }
            if line == "[pins]" {
                section = \.pins
                continue
            }
            guard let section, let equals = line.firstIndex(of: "=") else { continue }
            let key = unquote(String(line[..<equals]).trimmingCharacters(in: .whitespaces))
            let value = unquote(String(line[line.index(after: equals)...]).trimmingCharacters(in: .whitespaces))
            config[keyPath: section][key] = value
        }
        return config
    }

    func save(paths: Paths) throws {
        try FileManager.default.createDirectory(at: paths.configFile.deletingLastPathComponent(), withIntermediateDirectories: true)
        var lines: [String] = ["[defaults]"]
        for key in defaults.keys.sorted() {
            lines.append("\"\(key)\" = \"\(defaults[key] ?? "")\"")
        }
        lines.append("")
        lines.append("[pins]")
        for key in pins.keys.sorted() {
            lines.append("\"\(key)\" = \"\(pins[key] ?? "")\"")
        }
        lines.append("")
        try lines.joined(separator: "\n").write(to: paths.configFile, atomically: true, encoding: .utf8)
    }
}

private func unquote(_ value: String) -> String {
    if value.hasPrefix("\""), value.hasSuffix("\""), value.count >= 2 {
        return String(value.dropFirst().dropLast())
    }
    return value
}
