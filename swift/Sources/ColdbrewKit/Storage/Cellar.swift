import Foundation

public struct RuntimeDependencyRecord: Codable, Equatable, Sendable {
    public var name: String
    public var version: String
    public var path: String
}

public struct InstalledPackageRecord: Codable, Equatable, Sendable {
    public var name: String
    public var version: String
    public var tap: String
    public var cellarPath: String
    public var installedAt: String
    public var runtimeDependencies: [RuntimeDependencyRecord]
    public var linked: Bool
    public var pinned: Bool
    public var bottleTag: String?
    public var bottleSha256: String?
    public var kegOnly: Bool
    public var caveats: String?
    public var binaries: [String]
    public var installedAsDependency: Bool
    public var installedFor: String?

    enum CodingKeys: String, CodingKey {
        case name
        case version
        case tap
        case cellarPath = "cellar_path"
        case installedAt = "installed_at"
        case runtimeDependencies = "runtime_dependencies"
        case linked
        case pinned
        case bottleTag = "bottle_tag"
        case bottleSha256 = "bottle_sha256"
        case kegOnly = "keg_only"
        case caveats
        case binaries
        case installedAsDependency = "installed_as_dependency"
        case installedFor = "installed_for"
    }

    public init(
        name: String,
        version: String,
        tap: String = "homebrew/core",
        cellarPath: String,
        installedAt: String = ISO8601DateFormatter().string(from: Date()),
        runtimeDependencies: [RuntimeDependencyRecord] = [],
        linked: Bool = false,
        pinned: Bool = false,
        bottleTag: String? = nil,
        bottleSha256: String? = nil,
        kegOnly: Bool = false,
        caveats: String? = nil,
        binaries: [String] = [],
        installedAsDependency: Bool = false,
        installedFor: String? = nil
    ) {
        self.name = name
        self.version = version
        self.tap = tap
        self.cellarPath = cellarPath
        self.installedAt = installedAt
        self.runtimeDependencies = runtimeDependencies
        self.linked = linked
        self.pinned = pinned
        self.bottleTag = bottleTag
        self.bottleSha256 = bottleSha256
        self.kegOnly = kegOnly
        self.caveats = caveats
        self.binaries = binaries
        self.installedAsDependency = installedAsDependency
        self.installedFor = installedFor
    }
}

public struct InstallReceiptRecord: Codable, Equatable, Sendable {
    public var installedBy: String
    public var installedAt: String
    public var source: String
    public var checksumVerified: Bool

    enum CodingKeys: String, CodingKey {
        case installedBy = "installed_by"
        case installedAt = "installed_at"
        case source
        case checksumVerified = "checksum_verified"
    }

    public init(
        installedBy: String = "crew swift",
        installedAt: String = ISO8601DateFormatter().string(from: Date()),
        source: String,
        checksumVerified: Bool = true
    ) {
        self.installedBy = installedBy
        self.installedAt = installedAt
        self.source = source
        self.checksumVerified = checksumVerified
    }
}

public struct PackageMetadataRecord: Codable, Equatable, Sendable {
    public var package: InstalledPackageRecord
    public var formulaJson: JSONValue?
    public var receipt: InstallReceiptRecord

    enum CodingKeys: String, CodingKey {
        case package
        case formulaJson = "formula_json"
        case receipt
    }

    public init(package: InstalledPackageRecord, formulaJson: JSONValue? = nil, receipt: InstallReceiptRecord) {
        self.package = package
        self.formulaJson = formulaJson
        self.receipt = receipt
    }
}

public enum JSONValue: Codable, Equatable, Sendable {
    case null
    case bool(Bool)
    case number(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let bool = try? container.decode(Bool.self) {
            self = .bool(bool)
        } else if let number = try? container.decode(Double.self) {
            self = .number(number)
        } else if let string = try? container.decode(String.self) {
            self = .string(string)
        } else if let array = try? container.decode([JSONValue].self) {
            self = .array(array)
        } else {
            self = .object(try container.decode([String: JSONValue].self))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null:
            try container.encodeNil()
        case .bool(let value):
            try container.encode(value)
        case .number(let value):
            try container.encode(value)
        case .string(let value):
            try container.encode(value)
        case .array(let value):
            try container.encode(value)
        case .object(let value):
            try container.encode(value)
        }
    }
}

public struct Cellar: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    public func listPackages() throws -> [InstalledPackageRecord] {
        guard FileManager.default.fileExists(atPath: paths.cellarDir.path) else {
            return []
        }

        var packages: [InstalledPackageRecord] = []
        let names = try FileManager.default.contentsOfDirectory(at: paths.cellarDir, includingPropertiesForKeys: [.isDirectoryKey])
        for nameDir in names where try isDirectory(nameDir) {
            let versions = try FileManager.default.contentsOfDirectory(at: nameDir, includingPropertiesForKeys: [.isDirectoryKey])
            for versionDir in versions where try isDirectory(versionDir) {
                if let package = try? package(name: nameDir.lastPathComponent, version: versionDir.lastPathComponent) {
                    packages.append(package)
                }
            }
        }
        return packages.sorted { lhs, rhs in
            lhs.name == rhs.name ? lhs.version < rhs.version : lhs.name < rhs.name
        }
    }

    public func package(name: String, version: String) throws -> InstalledPackageRecord {
        let metadataURL = paths.packageMetadata(name: name, version: version)
        guard FileManager.default.fileExists(atPath: metadataURL.path) else {
            throw ColdbrewError.packageNotInstalled(name: name, version: version)
        }
        let data = try Data(contentsOf: metadataURL)
        return try JSONDecoder().decode(PackageMetadataRecord.self, from: data).package
    }

    public func versions(name: String) throws -> [String] {
        let packageDir = paths.cellarDir.appendingPathComponent(name, isDirectory: true)
        guard FileManager.default.fileExists(atPath: packageDir.path) else {
            return []
        }
        let entries = try FileManager.default.contentsOfDirectory(at: packageDir, includingPropertiesForKeys: [.isDirectoryKey])
        return try entries.filter { try isDirectory($0) }.map(\.lastPathComponent).sorted()
    }

    public func isInstalled(name: String, version: String) -> Bool {
        FileManager.default.fileExists(atPath: paths.cellarPackage(name, version: version).path)
    }

    public func packagePath(name: String, version: String) -> URL {
        paths.cellarPackage(name, version: version)
    }

    public func latestVersion(name: String) throws -> String? {
        try versions(name: name).last
    }

    public func uninstall(name: String, version: String) throws {
        let packageURL = paths.cellarPackage(name, version: version)
        guard FileManager.default.fileExists(atPath: packageURL.path) else {
            throw ColdbrewError.packageNotInstalled(name: name, version: version)
        }

        try FileManager.default.removeItem(at: packageURL)
        let packageDir = paths.cellarDir.appendingPathComponent(name, isDirectory: true)
        if (try? FileManager.default.contentsOfDirectory(atPath: packageDir.path).isEmpty) == true {
            try FileManager.default.removeItem(at: packageDir)
        }
    }

    public func saveMetadata(_ metadata: PackageMetadataRecord) throws {
        let url = paths.packageMetadata(name: metadata.package.name, version: metadata.package.version)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        try encoder.encode(metadata).write(to: url)
    }

    public func binaries(name: String, version: String) throws -> [String] {
        let binDir = paths.cellarPackage(name, version: version).appendingPathComponent("bin", isDirectory: true)
        guard FileManager.default.fileExists(atPath: binDir.path) else {
            return []
        }
        let entries = try FileManager.default.contentsOfDirectory(at: binDir, includingPropertiesForKeys: [.isRegularFileKey])
        return try entries.filter { try isRegularFile($0) }.map(\.lastPathComponent).sorted()
    }

    public func diskUsage() throws -> UInt64 {
        guard FileManager.default.fileExists(atPath: paths.cellarDir.path) else {
            return 0
        }
        return try directorySize(paths.cellarDir)
    }

    private func isDirectory(_ url: URL) throws -> Bool {
        try url.resourceValues(forKeys: [.isDirectoryKey]).isDirectory == true
    }

    private func isRegularFile(_ url: URL) throws -> Bool {
        try url.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true
    }
}
