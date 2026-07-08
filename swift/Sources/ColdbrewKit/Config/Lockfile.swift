import Foundation

public struct Lockfile: Equatable, Sendable {
    public var version: UInt32
    public var generatedAt: Date
    public var packages: [String: LockedPackage]
    public var configHash: String

    public init(
        version: UInt32 = 1,
        generatedAt: Date = Date(),
        packages: [String: LockedPackage] = [:],
        configHash: String
    ) {
        self.version = version
        self.generatedAt = generatedAt
        self.packages = packages
        self.configHash = configHash
    }

    public static func load(from path: URL) throws -> Lockfile {
        guard FileManager.default.fileExists(atPath: path.path) else {
            throw ConfigError.lockfileNotFound
        }
        return try parse(String(contentsOf: path, encoding: .utf8))
    }

    public func save(to path: URL) throws {
        try toml().write(to: path, atomically: true, encoding: .utf8)
    }

    public func isInSync(with config: ProjectConfig) -> Bool {
        configHash == Self.hash(config.tomlForHash())
    }

    public func packageVersions() -> [String: String] {
        packages.mapValues(\.version)
    }

    public static func hash(_ string: String) -> String {
        LockfileChecksum.hash(string)
    }

    static func parse(_ text: String) throws -> Lockfile {
        let document = try MiniTOML.parse(text)
        let root = document[""] ?? [:]
        let formatter = ISO8601DateFormatter()

        guard let version = int(root["version"]),
              let generatedAtString = string(root["generated_at"]),
              let generatedAt = formatter.date(from: generatedAtString),
              let configHash = string(root["config_hash"])
        else {
            throw ConfigError.invalidTOML("Invalid lockfile header")
        }

        var packages: [String: LockedPackage] = [:]
        for (section, table) in document where section.hasPrefix("packages.") {
            let name = String(section.dropFirst("packages.".count))
            packages[name] = LockedPackage(
                version: string(table["version"]) ?? "",
                sha256: string(table["sha256"]),
                bottleTag: string(table["bottle_tag"]),
                tap: string(table["tap"]) ?? "",
                dependencies: array(table["dependencies"]) ?? [],
                dev: bool(table["dev"]) ?? false
            )
        }

        return Lockfile(version: UInt32(version), generatedAt: generatedAt, packages: packages, configHash: configHash)
    }

    func toml() -> String {
        let formatter = ISO8601DateFormatter()
        var document: [String: MiniTOML.Table] = [
            "": [
                "version": .integer(Int(version)),
                "generated_at": .string(formatter.string(from: generatedAt)),
                "config_hash": .string(configHash),
            ]
        ]

        for (name, package) in packages {
            var table: MiniTOML.Table = [
                "version": .string(package.version),
                "tap": .string(package.tap),
                "dependencies": .array(package.dependencies),
                "dev": .bool(package.dev),
            ]
            if let sha256 = package.sha256 {
                table["sha256"] = .string(sha256)
            }
            if let bottleTag = package.bottleTag {
                table["bottle_tag"] = .string(bottleTag)
            }
            document["packages.\(name)"] = table
        }

        return MiniTOML.render(document, sectionOrder: [""] + packages.keys.sorted().map { "packages.\($0)" })
    }
}

public struct LockedPackage: Equatable, Sendable {
    public var version: String
    public var sha256: String?
    public var bottleTag: String?
    public var tap: String
    public var dependencies: [String]
    public var dev: Bool

    public init(
        version: String,
        sha256: String? = nil,
        bottleTag: String? = nil,
        tap: String,
        dependencies: [String] = [],
        dev: Bool = false
    ) {
        self.version = version
        self.sha256 = sha256
        self.bottleTag = bottleTag
        self.tap = tap
        self.dependencies = dependencies
        self.dev = dev
    }
}

extension ProjectConfig {
    public func tomlForHash() -> String {
        toml()
    }
}
