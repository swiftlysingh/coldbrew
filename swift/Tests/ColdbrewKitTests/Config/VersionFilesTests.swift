import Foundation
@testable import ColdbrewKit
import XCTest

final class VersionFilesTests: XCTestCase {
    func testDetectsNvmrcAndStripsVPrefix() throws {
        let directory = temporaryDirectory()
        try "v18.17.0\n".write(to: directory.appendingPathComponent(".nvmrc"), atomically: true, encoding: .utf8)

        let versions = try VersionFileDetector(startDirectory: directory).detectAll()

        XCTAssertEqual(versions.count, 1)
        XCTAssertEqual(versions[0].package, "node")
        XCTAssertEqual(versions[0].version, "18.17.0")
        XCTAssertEqual(versions[0].fileType, .nvmrc)
    }

    func testDetectsToolVersions() throws {
        let directory = temporaryDirectory()
        try """
        nodejs 18.17.0
        python 3.11.0
        # comment
        ruby 3.2.0
        """.write(to: directory.appendingPathComponent(".tool-versions"), atomically: true, encoding: .utf8)

        let versions = try VersionFileDetector(startDirectory: directory).detectAll()

        XCTAssertTrue(versions.contains { $0.package == "node" && $0.version == "18.17.0" })
        XCTAssertTrue(versions.contains { $0.package == "python" && $0.version == "3.11.0" })
        XCTAssertTrue(versions.contains { $0.package == "ruby" && $0.version == "3.2.0" })
    }

    func testVersionMapKeepsClosestVersion() throws {
        let root = temporaryDirectory()
        let child = root.appendingPathComponent("child")
        try FileManager.default.createDirectory(at: child, withIntermediateDirectories: true)
        try "18.0.0".write(to: root.appendingPathComponent(".nvmrc"), atomically: true, encoding: .utf8)
        try "20.0.0".write(to: child.appendingPathComponent(".nvmrc"), atomically: true, encoding: .utf8)

        let map = try getVersionMap(startDirectory: child)

        XCTAssertEqual(map["node"], "20.0.0")
    }
}
