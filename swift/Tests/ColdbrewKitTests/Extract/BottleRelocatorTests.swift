import Foundation
import Testing
@testable import ColdbrewKit

@Test func relocatorScansButIgnoresNonMachOPlaceholderFiles() throws {
    let root = temporaryDirectory()
    let install = root.appendingPathComponent("cellar/hello/1.0.0", isDirectory: true)
    try FileManager.default.createDirectory(at: install, withIntermediateDirectories: true)
    try Data("@@HOMEBREW_PREFIX@@".utf8).write(to: install.appendingPathComponent("text.txt"))

    let summary = try BottleRelocator(paths: Paths(root: root)).relocateBottle(at: install)

    #if os(macOS)
    #expect(summary.scannedFiles == 1)
    #else
    #expect(summary.scannedFiles == 0)
    #endif
    #expect(summary.machOFiles == 0)
    #expect(summary.relocatedFiles == 0)
    #expect(summary.unhandledPlaceholders == 0)
}

@Test func codesignTreeIgnoresDirectoriesWithoutMachOFiles() throws {
    let root = temporaryDirectory()
    let install = root.appendingPathComponent("cellar/hello/1.0.0", isDirectory: true)
    try FileManager.default.createDirectory(at: install, withIntermediateDirectories: true)
    try Data("plain text".utf8).write(to: install.appendingPathComponent("README"))

    #expect(try BottleRelocator(paths: Paths(root: root)).codesignMachOTree(at: install) == 0)
}
