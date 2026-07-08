import Foundation
import Testing
@testable import ColdbrewKit

@Test func homebrewAPILoadsFixtureIndexAndFormula() throws {
    let registry = try fixtureRegistry()
    let api = HomebrewAPI(baseURL: registry)

    let formulas = try api.fetchFormulaIndex()
    let hello = try api.fetchFormula(named: "hello")

    #expect(formulas.map(\.name).sorted() == ["dep", "hello", "uses-dep"])
    #expect(hello.name == "hello")
    #expect(hello.fullName == "homebrew/core/hello")
    #expect(hello.version == "1.0.0")
    #expect(hello.bottle.stable?.files["arm64_sonoma"]?.isRelocatable == true)

    do {
        _ = try api.fetchFormula(named: "missing")
        Issue.record("expected packageNotFound")
    } catch let error as ColdbrewError {
        #expect(error == .packageNotFound("missing"))
    }
}

@Test func formulaIndexUpdatesFromFixtureAndSearches() throws {
    let registry = try fixtureRegistry()
    let paths = Paths(root: temporaryDirectory())
    let index = FormulaIndex(paths: paths)

    #expect(!index.exists)
    #expect(try index.update(from: HomebrewAPI(baseURL: registry)) == 3)
    #expect(index.exists)

    #expect(try index.formula(named: "uses-dep")?.dependencies == ["dep"])
    #expect(try index.formula(named: "missing") == nil)
    #expect(try index.listFormulas().map(\.name) == ["dep", "hello", "uses-dep"])
    #expect(try index.search("dep").map(\.name) == ["dep", "uses-dep"])
}

@Test func formulaIndexRequiresUpdateBeforeLookup() throws {
    let index = FormulaIndex(indexURL: temporaryDirectory().appendingPathComponent("formula.json"))

    do {
        _ = try index.formula(named: "hello")
        Issue.record("expected indexNotInitialized")
    } catch let error as ColdbrewError {
        #expect(error == .indexNotInitialized)
    }
}

private func fixtureRegistry() throws -> URL {
    let root = temporaryDirectory()
    let formulaDirectory = root.appendingPathComponent("formula", isDirectory: true)
    try FileManager.default.createDirectory(at: formulaDirectory, withIntermediateDirectories: true)

    let formulas = [
        formulaJSON(name: "hello", desc: "Friendly greeting tool", version: "1.0.0"),
        formulaJSON(name: "dep", desc: "Dependency library", version: "2.0.0"),
        formulaJSON(name: "uses-dep", desc: "Tool using dependency", version: "3.0.0", dependencies: ["dep"]),
    ]

    try "[\(formulas.joined(separator: ","))]".write(
        to: root.appendingPathComponent("formula.json"),
        atomically: true,
        encoding: .utf8
    )
    try formulas[0].write(
        to: formulaDirectory.appendingPathComponent("hello.json"),
        atomically: true,
        encoding: .utf8
    )

    return root
}

private func formulaJSON(
    name: String,
    desc: String,
    version: String,
    dependencies: [String] = []
) -> String {
    let dependencies = dependencies.map { "\"\($0)\"" }.joined(separator: ",")
    return """
    {
      "name": "\(name)",
      "full_name": "homebrew/core/\(name)",
      "tap": "homebrew/core",
      "desc": "\(desc)",
      "homepage": "https://example.com/\(name)",
      "license": "MIT",
      "versions": {
        "stable": "\(version)",
        "bottle": true
      },
      "bottle": {
        "stable": {
          "root_url": "https://ghcr.io/v2/homebrew/core",
          "files": {
            "arm64_sonoma": {
              "cellar": ":any_skip_relocation",
              "url": "https://example.com/\(name).tar.gz",
              "sha256": "\(String(repeating: "a", count: 64))"
            }
          }
        }
      },
      "dependencies": [\(dependencies)],
      "unknown_homebrew_field": "ignored"
    }
    """
}
