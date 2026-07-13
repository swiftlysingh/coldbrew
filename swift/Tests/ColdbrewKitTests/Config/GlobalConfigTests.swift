import Foundation
import Testing
@testable import ColdbrewKit

@Test func defaultGlobalConfigMatchesRustDefaults() {
    let config = GlobalConfig()

    #expect(config.defaults.isEmpty)
    #expect(config.pins.isEmpty)
    #expect(config.settings.autoUpdate)
    #expect(!config.settings.cdnRacing)
    #expect(config.settings.parallelDownloads >= 2)
    #expect(config.settings.parallelDownloads <= 16)
    #expect(config.settings.parallelExtractions >= 1)
    #expect(config.settings.parallelExtractions <= 4)
    #expect(config.settings.parallelCodesigning >= 1)
    #expect(config.settings.parallelCodesigning <= 4)
    #expect(config.settings.parallelInstalls >= 1)
    #expect(config.settings.parallelInstalls <= 4)
    #expect(!config.settings.perBottleProgress)
}

@Test func globalConfigRoundTripsDefaultsAndPins() throws {
    let directory = temporaryDirectory()
    let path = directory.appendingPathComponent("config.toml")

    var config = GlobalConfig()
    config.setDefault("node", version: "22.0.0")
    config.addPin("jq", version: "1.7.1")

    try config.save(to: path)

    let loaded = try GlobalConfig.load(from: path)
    #expect(loaded.getDefault("node") == "22.0.0")
    #expect(loaded.isPinned("jq"))
    #expect(loaded.getPin("jq") == "1.7.1")
}
