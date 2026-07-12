import Foundation
import Testing
@testable import ColdbrewKit

@Test func doctorReportsMissingPathAndIndexWarnings() throws {
    let paths = Paths(root: temporaryDirectory())
    try paths.createDirectories()

    let report = DoctorChecker(paths: paths, environment: ["PATH": "/usr/bin"]).run()

    #expect(report.issues.isEmpty)
    #expect(report.warnings.contains("Package index not found. Run 'crew update'"))
    #expect(report.warnings.contains("Coldbrew bin directory not in PATH. Add \(paths.binDir.path) to your PATH."))
}
