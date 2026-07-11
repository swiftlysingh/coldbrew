import Foundation
import Testing
@testable import ColdbrewKit

@Test func shimManagerCreatesListsAndRemovesShims() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let manager = ShimManager(paths: paths)

    let shims = try manager.createShims(name: "hello", version: "1.0.0", binaries: ["hello"])

    #expect(shims == [paths.shim("hello")])
    #expect(manager.hasShim(binary: "hello"))
    let content = try String(contentsOf: paths.shim("hello"))
    #expect(content.contains("# Coldbrew shim for hello/hello"))
    #expect(content.contains("exec crew exec 'hello' 'hello'"))
    #expect(try manager.listShims().map(\.package) == ["hello"])

    try manager.removeShims(binaries: ["hello"])
    #expect(!manager.hasShim(binary: "hello"))
}

@Test func shimManagerShellQuotesPackageAndBinaryNames() throws {
    let paths = Paths(root: temporaryDirectory())
    try paths.createDirectories()

    try ShimManager(paths: paths).createShims(name: "hello package", version: "1.0.0", binaries: ["say'hi"])

    let content = try String(contentsOf: paths.shim("say'hi"))
    #expect(content.contains("exec crew exec 'hello package' 'say'\\''hi' \"$@\""))
}

@Test func shimManagerResolvesBinaryFromProjectThenDefaults() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let manager = ShimManager(paths: paths)
    let binDir = paths.cellarPackage("hello", version: "2.0.0").appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: binDir, withIntermediateDirectories: true)
    FileManager.default.createFile(atPath: binDir.appendingPathComponent("hello").path, contents: Data())

    let resolved = try manager.resolveBinary(
        package: "hello",
        binary: "hello",
        defaults: ["hello": "1.0.0"],
        projectVersions: ["hello": "2.0.0"]
    )

    #expect(resolved == binDir.appendingPathComponent("hello"))
}

@Test func shimManagerFallsBackToLatestInstalledVersion() throws {
    let paths = Paths(root: temporaryDirectory())
    let binary = paths.cellarPackage("hello", version: "2.0.0").appendingPathComponent("bin/hello")
    try FileManager.default.createDirectory(at: binary.deletingLastPathComponent(), withIntermediateDirectories: true)
    FileManager.default.createFile(atPath: binary.path, contents: Data())

    let resolved = try ShimManager(paths: paths).resolveBinary(
        package: "hello",
        binary: "hello",
        defaults: [:]
    )

    #expect(resolved == binary)
}
