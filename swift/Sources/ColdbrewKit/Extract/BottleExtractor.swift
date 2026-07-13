import Foundation

public struct ExtractionResult: Equatable, Sendable {
    public var path: URL
    public var sizeBytes: UInt64
}

public struct BottleExtractor: Sendable {
    public init() {}

    @discardableResult
    public func extract(_ archive: URL, to destination: URL, stripComponents: Int = 2) throws -> ExtractionResult {
        if FileManager.default.fileExists(atPath: destination.path) {
            try FileManager.default.removeItem(at: destination)
        }
        try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)

        var args = ["-xzf", archive.path, "-C", destination.path]
        if stripComponents > 0 {
            args.append("--strip-components")
            args.append(String(stripComponents))
        }
        try ProcessRunner.run("tar", args)
        return ExtractionResult(path: destination, sizeBytes: try directorySize(destination))
    }
}
