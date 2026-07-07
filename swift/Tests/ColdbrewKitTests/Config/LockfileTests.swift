import Foundation
import Testing
@testable import ColdbrewKit

@Test func lockfileHashMatchesSha256() {
    #expect(Lockfile.hash("test") == "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08")
    #expect(Lockfile.hash("test") != Lockfile.hash("different"))
}

@Test func lockfileRoundTripsPackages() throws {
    let path = temporaryDirectory().appendingPathComponent("coldbrew.lock")
    let generatedAt = Date(timeIntervalSince1970: 1_700_000_000)
    let lockfile = Lockfile(
        generatedAt: generatedAt,
        packages: [
            "jq": LockedPackage(
                version: "1.7.1",
                sha256: "abc123",
                bottleTag: "arm64_ventura",
                tap: "homebrew/core",
                dependencies: ["oniguruma"],
                dev: false
            )
        ],
        configHash: Lockfile.hash("config")
    )

    try lockfile.save(to: path)
    let loaded = try Lockfile.load(from: path)

    #expect(loaded.version == 1)
    #expect(loaded.packages["jq"]?.version == "1.7.1")
    #expect(loaded.packages["jq"]?.sha256 == "abc123")
    #expect(loaded.packages["jq"]?.bottleTag == "arm64_ventura")
    #expect(loaded.packages["jq"]?.dependencies == ["oniguruma"])
}

@Test func lockfileSyncUsesProjectConfigHash() {
    var config = ProjectConfig()
    config.addPackage("jq", version: "1.7.1", dev: false)

    let lockfile = Lockfile(configHash: Lockfile.hash(config.tomlForHash()))
    #expect(lockfile.isInSync(with: config))

    config.addPackage("node", version: "22", dev: true)
    #expect(!lockfile.isInSync(with: config))
}
