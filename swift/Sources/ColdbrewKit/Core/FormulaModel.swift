import Foundation

public struct Formula: Codable, Equatable, Sendable {
    public var name: String
    public var fullName: String
    public var tap: String
    public var desc: String?
    public var homepage: String?
    public var license: String?
    public var versions: Versions
    public var bottle: BottleSpec
    public var dependencies: [String]
    public var buildDependencies: [String]
    public var optionalDependencies: [String]
    public var testDependencies: [String]
    public var recommendedDependencies: [String]
    public var kegOnly: Bool
    public var revision: UInt32
    public var versionScheme: UInt32
    public var caveats: String?

    enum CodingKeys: String, CodingKey {
        case name
        case fullName = "full_name"
        case tap
        case desc
        case homepage
        case license
        case versions
        case bottle
        case dependencies
        case buildDependencies = "build_dependencies"
        case optionalDependencies = "optional_dependencies"
        case testDependencies = "test_dependencies"
        case recommendedDependencies = "recommended_dependencies"
        case kegOnly = "keg_only"
        case revision
        case versionScheme = "version_scheme"
        case caveats
    }

    public init(
        name: String,
        fullName: String? = nil,
        tap: String = "homebrew/core",
        desc: String? = nil,
        homepage: String? = nil,
        license: String? = nil,
        versions: Versions,
        bottle: BottleSpec = BottleSpec(),
        dependencies: [String] = [],
        buildDependencies: [String] = [],
        optionalDependencies: [String] = [],
        testDependencies: [String] = [],
        recommendedDependencies: [String] = [],
        kegOnly: Bool = false,
        revision: UInt32 = 0,
        versionScheme: UInt32 = 0,
        caveats: String? = nil
    ) {
        self.name = name
        self.fullName = fullName ?? "\(tap)/\(name)"
        self.tap = tap
        self.desc = desc
        self.homepage = homepage
        self.license = license
        self.versions = versions
        self.bottle = bottle
        self.dependencies = dependencies
        self.buildDependencies = buildDependencies
        self.optionalDependencies = optionalDependencies
        self.testDependencies = testDependencies
        self.recommendedDependencies = recommendedDependencies
        self.kegOnly = kegOnly
        self.revision = revision
        self.versionScheme = versionScheme
        self.caveats = caveats
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.name = try container.decode(String.self, forKey: .name)
        self.fullName = try container.decodeIfPresent(String.self, forKey: .fullName) ?? name
        self.tap = try container.decodeIfPresent(String.self, forKey: .tap) ?? "homebrew/core"
        self.desc = try container.decodeIfPresent(String.self, forKey: .desc)
        self.homepage = try container.decodeIfPresent(String.self, forKey: .homepage)
        self.license = try container.decodeIfPresent(String.self, forKey: .license)
        self.versions = try container.decode(Versions.self, forKey: .versions)
        self.bottle = try container.decodeIfPresent(BottleSpec.self, forKey: .bottle) ?? BottleSpec()
        self.dependencies = try container.decodeIfPresent([String].self, forKey: .dependencies) ?? []
        self.buildDependencies = try container.decodeIfPresent([String].self, forKey: .buildDependencies) ?? []
        self.optionalDependencies = try container.decodeIfPresent([String].self, forKey: .optionalDependencies) ?? []
        self.testDependencies = try container.decodeIfPresent([String].self, forKey: .testDependencies) ?? []
        self.recommendedDependencies = try container.decodeIfPresent([String].self, forKey: .recommendedDependencies) ?? []
        self.kegOnly = try container.decodeIfPresent(Bool.self, forKey: .kegOnly) ?? false
        self.revision = try container.decodeIfPresent(UInt32.self, forKey: .revision) ?? 0
        self.versionScheme = try container.decodeIfPresent(UInt32.self, forKey: .versionScheme) ?? 0
        self.caveats = try container.decodeIfPresent(String.self, forKey: .caveats)
    }

    public var version: String { versions.stable }

    public var versionWithRevision: String {
        revision > 0 ? "\(versions.stable)_\(revision)" : versions.stable
    }

    public var displayName: String {
        "\(name) \(versions.stable)"
    }

    public func hasBottle(tag: String) -> Bool {
        bottle.stable?.files[tag] != nil
    }

    public func bottle(forTag tag: String) -> BottleFile? {
        bottle.stable?.files[tag]
    }
}

public struct Versions: Codable, Equatable, Sendable {
    public var stable: String
    public var head: String?
    public var bottle: Bool

    public init(stable: String, head: String? = nil, bottle: Bool = false) {
        self.stable = stable
        self.head = head
        self.bottle = bottle
    }
}
