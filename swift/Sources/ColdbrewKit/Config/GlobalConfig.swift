import Foundation

public struct GlobalConfig: Equatable, Sendable {
    public var defaults: [String: String]
    public var pins: [String: String]
    public var settings: Settings

    public init(
        defaults: [String: String] = [:],
        pins: [String: String] = [:],
        settings: Settings = Settings()
    ) {
        self.defaults = defaults
        self.pins = pins
        self.settings = settings
    }

    public static func load(from path: URL) throws -> GlobalConfig {
        guard FileManager.default.fileExists(atPath: path.path) else {
            return GlobalConfig()
        }

        return try parse(String(contentsOf: path, encoding: .utf8))
    }

    public func save(to path: URL) throws {
        try FileManager.default.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
        try toml().write(to: path, atomically: true, encoding: .utf8)
    }

    public func getDefault(_ name: String) -> String? {
        defaults[name]
    }

    public mutating func setDefault(_ name: String, version: String) {
        defaults[name] = version
    }

    public mutating func removeDefault(_ name: String) {
        defaults.removeValue(forKey: name)
    }

    public func isPinned(_ name: String) -> Bool {
        pins[name] != nil
    }

    public func getPin(_ name: String) -> String? {
        pins[name]
    }

    public mutating func addPin(_ name: String, version: String) {
        pins[name] = version
    }

    public mutating func removePin(_ name: String) {
        pins.removeValue(forKey: name)
    }

    static func parse(_ text: String) throws -> GlobalConfig {
        let document = try MiniTOML.parse(text)
        let defaults = stringMap(document["defaults"])
        let pins = stringMap(document["pins"])
        let settings = Settings.parse(document["settings"] ?? [:])
        return GlobalConfig(defaults: defaults, pins: pins, settings: settings)
    }

    func toml() -> String {
        MiniTOML.render([
            "defaults": defaults.mapValues { .string($0) },
            "pins": pins.mapValues { .string($0) },
            "settings": settings.toml(),
        ], sectionOrder: ["defaults", "pins", "settings"])
    }
}

public struct Settings: Equatable, Sendable {
    public var autoUpdate: Bool
    public var parallelDownloads: Int
    public var parallelExtractions: Int
    public var parallelCodesigning: Int
    public var parallelInstalls: Int
    public var perBottleProgress: Bool
    public var keepVersions: Int
    public var analytics: Bool
    public var cdnRacing: Bool

    public init(
        autoUpdate: Bool = true,
        parallelDownloads: Int = Settings.defaultParallelDownloads,
        parallelExtractions: Int = Settings.defaultParallelExtractions,
        parallelCodesigning: Int = Settings.defaultParallelCodesigning,
        parallelInstalls: Int = Settings.defaultParallelInstalls,
        perBottleProgress: Bool = false,
        keepVersions: Int = 2,
        analytics: Bool = false,
        cdnRacing: Bool = false
    ) {
        self.autoUpdate = autoUpdate
        self.parallelDownloads = parallelDownloads
        self.parallelExtractions = parallelExtractions
        self.parallelCodesigning = parallelCodesigning
        self.parallelInstalls = parallelInstalls
        self.perBottleProgress = perBottleProgress
        self.keepVersions = keepVersions
        self.analytics = analytics
        self.cdnRacing = cdnRacing
    }

    private static var processors: Int {
        max(1, ProcessInfo.processInfo.activeProcessorCount)
    }

    public static var defaultParallelDownloads: Int {
        min(max(processors * 2, 2), 16)
    }

    public static var defaultParallelExtractions: Int {
        min(max(processors - 1, 1), 4)
    }

    public static var defaultParallelCodesigning: Int {
        min(max(processors, 1), 4)
    }

    public static var defaultParallelInstalls: Int {
        min(max(processors - 1, 1), 4)
    }

    static func parse(_ table: MiniTOML.Table) -> Settings {
        Settings(
            autoUpdate: bool(table["auto_update"]) ?? true,
            parallelDownloads: int(table["parallel_downloads"]) ?? defaultParallelDownloads,
            parallelExtractions: int(table["parallel_extractions"]) ?? defaultParallelExtractions,
            parallelCodesigning: int(table["parallel_codesigning"]) ?? defaultParallelCodesigning,
            parallelInstalls: int(table["parallel_installs"]) ?? defaultParallelInstalls,
            perBottleProgress: bool(table["per_bottle_progress"]) ?? false,
            keepVersions: int(table["keep_versions"]) ?? 2,
            analytics: bool(table["analytics"]) ?? false,
            cdnRacing: bool(table["cdn_racing"]) ?? false
        )
    }

    func toml() -> MiniTOML.Table {
        [
            "auto_update": .bool(autoUpdate),
            "parallel_downloads": .integer(parallelDownloads),
            "parallel_extractions": .integer(parallelExtractions),
            "parallel_codesigning": .integer(parallelCodesigning),
            "parallel_installs": .integer(parallelInstalls),
            "per_bottle_progress": .bool(perBottleProgress),
            "keep_versions": .integer(keepVersions),
            "analytics": .bool(analytics),
            "cdn_racing": .bool(cdnRacing),
        ]
    }
}

func stringMap(_ table: MiniTOML.Table?) -> [String: String] {
    (table ?? [:]).compactMapValues(string)
}

func string(_ value: MiniTOML.Value?) -> String? {
    guard case .string(let string) = value else { return nil }
    return string
}

func bool(_ value: MiniTOML.Value?) -> Bool? {
    guard case .bool(let bool) = value else { return nil }
    return bool
}

func int(_ value: MiniTOML.Value?) -> Int? {
    guard case .integer(let int) = value else { return nil }
    return int
}

func array(_ value: MiniTOML.Value?) -> [String]? {
    guard case .array(let array) = value else { return nil }
    return array
}
