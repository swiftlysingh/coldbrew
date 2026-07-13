import Testing
@testable import ColdbrewKit

@Test func macOSBottleTagsMapSupportedMajorVersions() {
    let expectedTags = [
        (26, "tahoe"),
        (15, "sequoia"),
        (14, "sonoma"),
        (13, "ventura"),
        (12, "monterey"),
        (11, "big_sur"),
        (10, "catalina"),
    ]

    for (majorVersion, expectedTag) in expectedTags {
        let tags = Platform(
            os: .macOS,
            architecture: .arm64,
            macOSMajorVersion: majorVersion
        ).bottleTags

        #expect(tags.first == "arm64_\(expectedTag)")
        #expect(tags.last == "all")
    }
}

@Test func bottleTagsKeepCurrentMacOSFirstAndAlwaysTryUniversal() {
    #expect(
        Platform(os: .macOS, architecture: .arm64, macOSMajorVersion: 26).bottleTags
            == ["arm64_tahoe", "arm64_sequoia", "arm64_sonoma", "arm64_ventura", "all"]
    )
    #expect(
        Platform(os: .macOS, architecture: .x86_64, macOSMajorVersion: 14).bottleTags
            == ["sonoma", "sequoia", "ventura", "all"]
    )
    #expect(Platform(os: .linux, architecture: .arm64).bottleTags == ["arm64_linux", "all"])
    #expect(Platform(os: .linux, architecture: .x86_64).bottleTags == ["x86_64_linux", "all"])
}
