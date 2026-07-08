import Foundation

public struct VersionFileDetector: Sendable {
    public let startDirectory: URL

    public init(startDirectory: URL) {
        self.startDirectory = startDirectory
    }

    public func detectAll() throws -> [DetectedVersion] {
        var versions: [DetectedVersion] = []
        var current = startDirectory

        while true {
            try appendSimpleVersion(&versions, directory: current, filename: ".nvmrc", package: "node", fileType: .nvmrc)
            try appendSimpleVersion(&versions, directory: current, filename: ".node-version", package: "node", fileType: .nodeVersion)
            try appendSimpleVersion(&versions, directory: current, filename: ".python-version", package: "python", fileType: .pythonVersion)
            try appendSimpleVersion(&versions, directory: current, filename: ".ruby-version", package: "ruby", fileType: .rubyVersion)

            let toolVersions = current.appendingPathComponent(".tool-versions")
            if FileManager.default.fileExists(atPath: toolVersions.path) {
                versions += try parseToolVersions(toolVersions)
            }

            let parent = current.deletingLastPathComponent()
            if parent.path == current.path {
                break
            }
            current = parent
        }

        return versions
    }

    public func detect(forPackage package: String) throws -> DetectedVersion? {
        try detectAll().first { $0.package == package }
    }

    private func appendSimpleVersion(
        _ versions: inout [DetectedVersion],
        directory: URL,
        filename: String,
        package: String,
        fileType: VersionFileType
    ) throws {
        let path = directory.appendingPathComponent(filename)
        guard FileManager.default.fileExists(atPath: path.path),
              let version = try parseSimpleVersionFile(path)
        else {
            return
        }

        versions.append(DetectedVersion(package: package, version: version, filePath: path, fileType: fileType))
    }

    private func parseSimpleVersionFile(_ path: URL) throws -> String? {
        let version = try String(contentsOf: path, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !version.isEmpty else { return nil }
        return version.hasPrefix("v") ? String(version.dropFirst()) : version
    }

    private func parseToolVersions(_ path: URL) throws -> [DetectedVersion] {
        let content = try String(contentsOf: path, encoding: .utf8)
        var versions: [DetectedVersion] = []

        for rawLine in content.split(whereSeparator: \.isNewline) {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }

            let parts = line.split(whereSeparator: \.isWhitespace)
            guard parts.count >= 2 else { continue }

            versions.append(DetectedVersion(
                package: Self.packageName(forTool: String(parts[0])),
                version: String(parts[1]),
                filePath: path,
                fileType: .toolVersions
            ))
        }

        return versions
    }

    private static func packageName(forTool tool: String) -> String {
        switch tool {
        case "nodejs", "node":
            return "node"
        case "golang", "go":
            return "go"
        default:
            return tool
        }
    }
}

public struct DetectedVersion: Equatable, Sendable {
    public var package: String
    public var version: String
    public var filePath: URL
    public var fileType: VersionFileType

    public init(package: String, version: String, filePath: URL, fileType: VersionFileType) {
        self.package = package
        self.version = version
        self.filePath = filePath
        self.fileType = fileType
    }
}

public enum VersionFileType: Equatable, Sendable {
    case nvmrc
    case nodeVersion
    case pythonVersion
    case rubyVersion
    case toolVersions
}

public func getVersionMap(startDirectory: URL) throws -> [String: String] {
    var result: [String: String] = [:]
    for version in try VersionFileDetector(startDirectory: startDirectory).detectAll() {
        if result[version.package] == nil {
            result[version.package] = version.version
        }
    }
    return result
}
