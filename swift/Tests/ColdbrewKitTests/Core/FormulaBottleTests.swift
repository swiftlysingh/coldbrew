import Foundation
import Testing
@testable import ColdbrewKit

@Test func formulaDecodesHomebrewShape() throws {
    let json = """
    {
      "name": "jq",
      "full_name": "homebrew/core/jq",
      "tap": "homebrew/core",
      "desc": "Command-line JSON processor",
      "versions": { "stable": "1.7.1", "bottle": true },
      "dependencies": ["oniguruma"],
      "revision": 1,
      "bottle": {
        "stable": {
          "root_url": "https://ghcr.io/v2/homebrew/core",
          "files": {
            "arm64_sonoma": {
              "cellar": ":any",
              "url": "https://example.test/jq.tar.gz",
              "sha256": "abc123"
            }
          }
        }
      }
    }
    """.data(using: .utf8)!

    let formula = try JSONDecoder().decode(Formula.self, from: json)

    #expect(formula.name == "jq")
    #expect(formula.versionWithRevision == "1.7.1_1")
    #expect(formula.displayName == "jq 1.7.1")
    #expect(formula.dependencies == ["oniguruma"])
    #expect(formula.hasBottle(tag: "arm64_sonoma"))
    #expect(formula.bottle(forTag: "arm64_sonoma")?.cellar.isRelocatable == true)
}

@Test func bottleChoosesFirstMatchingPlatformTag() {
    let file = BottleFile(cellar: ":any", url: "https://example.test/pkg.tar.gz", sha256: "abc123")
    let files = BottleFiles(rootURL: "https://ghcr.io", files: ["arm64_sonoma": file])

    let best = files.bestForPlatform(tags: ["arm64_sequoia", "arm64_sonoma"])

    #expect(best?.tag == "arm64_sonoma")
    #expect(best?.file.ghcrURL(name: "jq") == "https://ghcr.io/v2/homebrew/core/jq/blobs/sha256:abc123")
}

@Test func everyPlatformFallsBackToUniversalBottles() {
    #expect(Platform(os: .macOS, architecture: .arm64).bottleTags.last == "all")
    #expect(Platform(os: .linux, architecture: .x86_64).bottleTags.last == "all")
}
