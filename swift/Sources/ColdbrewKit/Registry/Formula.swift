import Foundation

public struct Formula: Codable, Equatable, Sendable {
    public var name: String
    public var fullName: String
    public var tap: String
    public var desc: String?
    public var homepage: String?
    public var license: String?
    public var versions: FormulaVersions
    public var bottle: BottleSpec
    public var dependencies: [String]
    public var buildDependencies: [String]
    public var optionalDependencies: [String]
    public var testDependencies: [String]
    public var recommendedDependencies: [String]
    public var kegOnly: Bool
    public var revision: UInt32
    public var versionScheme: UInt32

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
    }

    public init(
        name: String,
        fullName: String,
        tap: String = "homebrew/core",
        desc: String? = nil,
        homepage: String? = nil,
        license: String? = nil,
        versions: FormulaVersions,
        bottle: BottleSpec = BottleSpec(),
        dependencies: [String] = [],
        buildDependencies: [String] = [],
        optionalDependencies: [String] = [],
        testDependencies: [String] = [],
        recommendedDependencies: [String] = [],
        kegOnly: Bool = false,
        revision: UInt32 = 0,
        versionScheme: UInt32 = 0
    ) {
        self.name = name
        self.fullName = fullName
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
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        name = try container.decode(String.self, forKey: .name)
        fullName = try container.decodeIfPresent(String.self, forKey: .fullName) ?? name
        tap = try container.decodeIfPresent(String.self, forKey: .tap) ?? "homebrew/core"
        desc = try container.decodeIfPresent(String.self, forKey: .desc)
        homepage = try container.decodeIfPresent(String.self, forKey: .homepage)
        license = try container.decodeIfPresent(String.self, forKey: .license)
        versions = try container.decode(FormulaVersions.self, forKey: .versions)
        bottle = try container.decodeIfPresent(BottleSpec.self, forKey: .bottle) ?? BottleSpec()
        dependencies = try container.decodeIfPresent([String].self, forKey: .dependencies) ?? []
        buildDependencies = try container.decodeIfPresent([String].self, forKey: .buildDependencies) ?? []
        optionalDependencies = try container.decodeIfPresent([String].self, forKey: .optionalDependencies) ?? []
        testDependencies = try container.decodeIfPresent([String].self, forKey: .testDependencies) ?? []
        recommendedDependencies = try container.decodeIfPresent([String].self, forKey: .recommendedDependencies) ?? []
        kegOnly = try container.decodeIfPresent(Bool.self, forKey: .kegOnly) ?? false
        revision = try container.decodeIfPresent(UInt32.self, forKey: .revision) ?? 0
        versionScheme = try container.decodeIfPresent(UInt32.self, forKey: .versionScheme) ?? 0
    }

    public var version: String {
        versions.stable
    }

    public var versionWithRevision: String {
        revision > 0 ? "\(versions.stable)_\(revision)" : versions.stable
    }
}

public struct FormulaVersions: Codable, Equatable, Sendable {
    public var stable: String
    public var head: String?
    public var bottle: Bool

    public init(stable: String, head: String? = nil, bottle: Bool = false) {
        self.stable = stable
        self.head = head
        self.bottle = bottle
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        stable = try container.decode(String.self, forKey: .stable)
        head = try container.decodeIfPresent(String.self, forKey: .head)
        bottle = try container.decodeIfPresent(Bool.self, forKey: .bottle) ?? false
    }
}

public struct BottleSpec: Codable, Equatable, Sendable {
    public var stable: BottleFiles?

    public init(stable: BottleFiles? = nil) {
        self.stable = stable
    }
}

public struct BottleFiles: Codable, Equatable, Sendable {
    public var rebuild: UInt32
    public var rootURL: String
    public var files: [String: BottleFile]

    enum CodingKeys: String, CodingKey {
        case rebuild
        case rootURL = "root_url"
        case files
    }

    public init(rebuild: UInt32 = 0, rootURL: String, files: [String: BottleFile]) {
        self.rebuild = rebuild
        self.rootURL = rootURL
        self.files = files
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        rebuild = try container.decodeIfPresent(UInt32.self, forKey: .rebuild) ?? 0
        rootURL = try container.decode(String.self, forKey: .rootURL)
        files = try container.decode([String: BottleFile].self, forKey: .files)
    }
}

public struct BottleFile: Codable, Equatable, Sendable {
    public var cellar: String
    public var url: String
    public var sha256: String

    public init(cellar: String, url: String, sha256: String) {
        self.cellar = cellar
        self.url = url
        self.sha256 = sha256
    }

    public var isRelocatable: Bool {
        cellar == ":any" || cellar == ":any_skip_relocation"
    }
}
