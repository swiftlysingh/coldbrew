import Foundation

public struct ProjectConfig: Equatable, Sendable {
    public var name: String?
    public var packages: [String: PackageSpec]
    public var devPackages: [String: PackageSpec]

    public init(
        name: String? = nil,
        packages: [String: PackageSpec] = [:],
        devPackages: [String: PackageSpec] = [:]
    ) {
        self.name = name
        self.packages = packages
        self.devPackages = devPackages
    }

    public static func load(from path: URL) throws -> ProjectConfig {
        try parse(String(contentsOf: path, encoding: .utf8))
    }

    public func save(to path: URL) throws {
        try toml().write(to: path, atomically: true, encoding: .utf8)
    }

    public func allPackages() -> [String: String] {
        var result = packages.mapValues(\.version)
        for (name, spec) in devPackages {
            result[name] = spec.version
        }
        return result
    }

    public mutating func addPackage(_ name: String, version: String, dev: Bool) {
        if dev {
            devPackages[name] = .version(version)
        } else {
            packages[name] = .version(version)
        }
    }

    public mutating func removePackage(_ name: String) {
        packages.removeValue(forKey: name)
        devPackages.removeValue(forKey: name)
    }

    static func parse(_ text: String) throws -> ProjectConfig {
        let document = try MiniTOML.parse(text)
        return ProjectConfig(
            name: string(document[""]?["name"]),
            packages: try packageMap(document["packages"]),
            devPackages: try packageMap(document["dev_packages"])
        )
    }

    func toml() -> String {
        var document: [String: MiniTOML.Table] = [:]
        if let name {
            document[""] = ["name": .string(name)]
        }
        document["packages"] = packages.mapValues(\.tomlValue)
        document["dev_packages"] = devPackages.mapValues(\.tomlValue)
        return MiniTOML.render(document, sectionOrder: ["packages", "dev_packages"])
    }

    private static func packageMap(_ table: MiniTOML.Table?) throws -> [String: PackageSpec] {
        var result: [String: PackageSpec] = [:]
        for (name, value) in table ?? [:] {
            switch value {
            case .string(let version):
                result[name] = .version(version)
            case .inlineTable(let table):
                guard let version = string(table["version"]) else {
                    throw ConfigError.invalidTOML("Package \(name) is missing version")
                }
                result[name] = .full(PackageSpecFull(
                    version: version,
                    tap: string(table["tap"]),
                    skipLink: bool(table["skip_link"]) ?? false
                ))
            default:
                throw ConfigError.invalidTOML("Invalid package spec for \(name)")
            }
        }
        return result
    }
}

public enum PackageSpec: Equatable, Sendable {
    case version(String)
    case full(PackageSpecFull)

    public var version: String {
        switch self {
        case .version(let version):
            return version
        case .full(let full):
            return full.version
        }
    }

    public var tap: String? {
        switch self {
        case .version:
            return nil
        case .full(let full):
            return full.tap
        }
    }

    public var skipLink: Bool {
        switch self {
        case .version:
            return false
        case .full(let full):
            return full.skipLink
        }
    }

    var tomlValue: MiniTOML.Value {
        switch self {
        case .version(let version):
            return .string(version)
        case .full(let full):
            var table: MiniTOML.Table = [
                "version": .string(full.version),
                "skip_link": .bool(full.skipLink),
            ]
            if let tap = full.tap {
                table["tap"] = .string(tap)
            }
            return .inlineTable(table)
        }
    }
}

public struct PackageSpecFull: Equatable, Sendable {
    public var version: String
    public var tap: String?
    public var skipLink: Bool

    public init(version: String, tap: String? = nil, skipLink: Bool = false) {
        self.version = version
        self.tap = tap
        self.skipLink = skipLink
    }
}
