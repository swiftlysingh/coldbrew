import Foundation
import Testing
@testable import ColdbrewKit

@Test func versionFilesDetectsNvmrcAndStripsVPrefix() throws {
    let directory = temporaryDirectory()
    try "v18.17.0\n".write(to: directory.appendingPathComponent(".nvmrc"), atomically: true, encoding: .utf8)

    let versions = try VersionFileDetector(startDirectory: directory).detectAll()

    #expect(versions.count == 1)
    #expect(versions[0].package == "node")
    #expect(versions[0].version == "18.17.0")
    #expect(versions[0].fileType == .nvmrc)
}

@Test func versionFilesDetectsToolVersions() throws {
    let directory = temporaryDirectory()
    try """
    nodejs 18.17.0
    python 3.11.0
    # comment
    ruby 3.2.0
    """.write(to: directory.appendingPathComponent(".tool-versions"), atomically: true, encoding: .utf8)

    let versions = try VersionFileDetector(startDirectory: directory).detectAll()

    #expect(versions.contains { $0.package == "node" && $0.version == "18.17.0" })
    #expect(versions.contains { $0.package == "python" && $0.version == "3.11.0" })
    #expect(versions.contains { $0.package == "ruby" && $0.version == "3.2.0" })
}

@Test func versionMapKeepsClosestVersion() throws {
    let root = temporaryDirectory()
    let child = root.appendingPathComponent("child")
    try FileManager.default.createDirectory(at: child, withIntermediateDirectories: true)
    try "18.0.0".write(to: root.appendingPathComponent(".nvmrc"), atomically: true, encoding: .utf8)
    try "20.0.0".write(to: child.appendingPathComponent(".nvmrc"), atomically: true, encoding: .utf8)

    let map = try getVersionMap(startDirectory: child)

    #expect(map["node"] == "20.0.0")
}
