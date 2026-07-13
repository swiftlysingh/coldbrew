import Testing
@testable import ColdbrewKit

@Test func macOSBottleTagsStartWithDetectedVersionAndRetainFallbacks() {
    #expect(
        Platform(os: .macOS, architecture: .arm64, macOSMajorVersion: 26).bottleTags
            == ["arm64_tahoe", "arm64_sequoia", "arm64_sonoma", "arm64_ventura", "all"]
    )
    #expect(
        Platform(os: .macOS, architecture: .x86_64, macOSMajorVersion: 14).bottleTags
            == ["sonoma", "sequoia", "ventura", "all"]
    )
}
