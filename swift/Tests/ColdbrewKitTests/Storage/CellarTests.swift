import Foundation
import Testing
@testable import ColdbrewKit

@Test func cellarSavesAndListsRustCompatiblePackageMetadata() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let cellar = Cellar(paths: paths)

    let package = InstalledPackageRecord(
        name: "hello",
        version: "1.0.0",
        cellarPath: paths.cellarPackage("hello", version: "1.0.0").path,
        binaries: ["hello"]
    )
    let metadata = PackageMetadataRecord(
        package: package,
        receipt: InstallReceiptRecord(source: "fixture")
    )

    try cellar.saveMetadata(metadata)

    #expect(try cellar.package(name: "hello", version: "1.0.0") == package)
    #expect(try cellar.versions(name: "hello") == ["1.0.0"])
    #expect(try cellar.latestVersion(name: "hello") == "1.0.0")
    #expect(try cellar.listPackages().map(\.name) == ["hello"])
}

@Test func cellarFindsBinariesAndUninstallsPackageVersion() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let cellar = Cellar(paths: paths)
    let packageDir = paths.cellarPackage("hello", version: "1.0.0")
    let binDir = packageDir.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: binDir, withIntermediateDirectories: true)
    FileManager.default.createFile(atPath: binDir.appendingPathComponent("hello").path, contents: Data())

    #expect(cellar.isInstalled(name: "hello", version: "1.0.0"))
    #expect(try cellar.binaries(name: "hello", version: "1.0.0") == ["hello"])

    try cellar.uninstall(name: "hello", version: "1.0.0")
    #expect(!cellar.isInstalled(name: "hello", version: "1.0.0"))
}
