import Foundation

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
        self.rebuild = try container.decodeIfPresent(UInt32.self, forKey: .rebuild) ?? 0
        self.rootURL = try container.decode(String.self, forKey: .rootURL)
        self.files = try container.decode([String: BottleFile].self, forKey: .files)
    }

    public func bestForPlatform(tags: [String]) -> (tag: String, file: BottleFile)? {
        for tag in tags {
            if let file = files[tag] {
                return (tag, file)
            }
        }
        return nil
    }
}

public struct BottleFile: Codable, Equatable, Sendable {
    public var cellar: CellarType
    public var url: String
    public var sha256: String

    public init(cellar: CellarType, url: String, sha256: String) {
        self.cellar = cellar
        self.url = url
        self.sha256 = sha256
    }

    public func ghcrURL(name: String) -> String {
        "https://ghcr.io/v2/homebrew/core/\(name)/blobs/sha256:\(sha256)"
    }

    public var isRelocatable: Bool {
        cellar.isRelocatable
    }
}

public struct CellarType: Codable, Equatable, ExpressibleByStringLiteral, Sendable {
    public var rawValue: String

    public init(_ rawValue: String = ":any_skip_relocation") {
        self.rawValue = rawValue
    }

    public init(stringLiteral value: String) {
        self.rawValue = value
    }

    public var isRelocatable: Bool {
        rawValue == ":any" || rawValue == ":any_skip_relocation"
    }

    public init(from decoder: Decoder) throws {
        self.rawValue = try decoder.singleValueContainer().decode(String.self)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }
}
