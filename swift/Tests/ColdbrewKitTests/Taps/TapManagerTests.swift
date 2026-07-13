import Foundation
import Testing
@testable import ColdbrewKit

@Test func tapManagerParsesHomebrewTapNamesLikeRust() throws {
    let parsed = try TapManager.parse("user/core")
    #expect(parsed.user == "user")
    #expect(parsed.repo == "homebrew-core")

    let alreadyPrefixed = try TapManager.parse("user/homebrew-extra")
    #expect(alreadyPrefixed.repo == "homebrew-extra")
}

@Test func tapManagerListsAndRemovesInstalledTaps() throws {
    let paths = Paths(root: temporaryDirectory())
    let tapPath = paths.tapDir(user: "user", repo: "homebrew-core")
    try FileManager.default.createDirectory(at: tapPath, withIntermediateDirectories: true)
    let manager = TapManager(paths: paths)

    #expect(try manager.list().map(\.fullName) == ["user/homebrew-core"])

    try manager.remove("user/core")
    #expect(try manager.list().isEmpty)
}
