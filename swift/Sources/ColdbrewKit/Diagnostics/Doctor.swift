import Foundation

public struct DoctorReport: Equatable, Sendable {
    public var issues: [String]
    public var warnings: [String]

    public var isHealthy: Bool {
        issues.isEmpty && warnings.isEmpty
    }
}

public struct DoctorChecker: Sendable {
    public let paths: Paths
    public var environment: [String: String]

    public init(paths: Paths, environment: [String: String] = ProcessInfo.processInfo.environment) {
        self.paths = paths
        self.environment = environment
    }

    public func run() -> DoctorReport {
        var issues: [String] = []
        var warnings: [String] = []

        if let path = environment["PATH"], path.split(separator: ":").map(String.init).contains(paths.binDir.path) {
            // OK
        } else {
            warnings.append("Coldbrew bin directory not in PATH. Add \(paths.binDir.path) to your PATH.")
        }

        for (label, directory) in [("Root", paths.root), ("Cellar", paths.cellarDir), ("Cache", paths.cacheDir)] {
            if FileManager.default.fileExists(atPath: directory.path),
               !FileManager.default.isWritableFile(atPath: directory.path) {
                issues.append("\(label) directory is not writable")
            }
        }

        if !FileManager.default.fileExists(atPath: paths.formulaIndex.path) {
            warnings.append("Package index not found. Run 'crew update'")
        }

        return DoctorReport(issues: issues.sorted(), warnings: warnings.sorted())
    }
}
