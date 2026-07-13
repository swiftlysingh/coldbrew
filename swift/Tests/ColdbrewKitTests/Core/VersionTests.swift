import Testing
@testable import ColdbrewKit

@Test func versionComparisonMatchesRustOrdering() throws {
    #expect(try Version("1.7.1") < Version("1.7.2"))
    #expect(try Version("1.7.2") < Version("1.8.0"))
    #expect(try Version("1.8.0") < Version("2.0.0"))
    #expect(try Version("1.0-beta") > Version("1.0"))
    #expect(try Version("1.0") < Version("1.0-rc1"))
}

@Test func versionComponentsAndMatchingMirrorRust() throws {
    let version = try Version("22.1.5")

    #expect(version.major == 22)
    #expect(version.minor == 1)
    #expect(version.patch == 5)
    #expect(versionMatches(version, constraint: "22"))
    #expect(versionMatches(version, constraint: "22.1"))
    #expect(!versionMatches(version, constraint: "21"))
}

@Test func packageSpecParsingMirrorsRust() {
    #expect(parsePackageSpec("jq") == (name: "jq", version: nil))
    #expect(parsePackageSpec("node@22") == (name: "node", version: "22"))
}
