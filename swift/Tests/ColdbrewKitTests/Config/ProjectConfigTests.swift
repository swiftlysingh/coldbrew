import Foundation
import Testing
@testable import ColdbrewKit

@Test func projectConfigRoundTripsPackages() throws {
    let path = temporaryDirectory().appendingPathComponent("coldbrew.toml")
    var config = ProjectConfig(name: "test-project")

    config.addPackage("jq", version: "1.7", dev: false)
    config.addPackage("node", version: "22", dev: true)

    try config.save(to: path)
    let loaded = try ProjectConfig.load(from: path)

    #expect(loaded.name == "test-project")
    #expect(loaded.packages["jq"]?.version == "1.7")
    #expect(loaded.devPackages["node"]?.version == "22")
}

@Test func projectConfigQuotesNonBarePackageNames() throws {
    var config = ProjectConfig()
    config.addPackage("python@3.12", version: "3.12.11", dev: false)

    let toml = config.toml()

    #expect(toml.contains("\"python@3.12\" = \"3.12.11\""))
    #expect(try ProjectConfig.parse(toml).packages["python@3.12"]?.version == "3.12.11")
}

@Test func packageSpecFullExposesFields() {
    let spec = PackageSpec.full(PackageSpecFull(version: "1.7.1", tap: "user/repo", skipLink: true))

    #expect(spec.version == "1.7.1")
    #expect(spec.tap == "user/repo")
    #expect(spec.skipLink)
}

@Test func projectConfigParsesInlinePackageSpec() throws {
    let config = try ProjectConfig.parse("""
    name = "demo"

    [packages]
    jq = { version = "1.7.1", tap = "homebrew/core", skip_link = true }
    """)

    #expect(config.packages["jq"]?.version == "1.7.1")
    #expect(config.packages["jq"]?.tap == "homebrew/core")
    #expect(config.packages["jq"]?.skipLink == true)
}
