import Testing
@testable import ColdbrewKit

@Test func outputPrefixesMatchRustContract() {
    var stdout: [String] = []
    var stderr: [String] = []
    let output = Output(
        verbose: true,
        stdout: { stdout.append($0) },
        stderr: { stderr.append($0) }
    )

    output.info("Installing jq")
    output.success("Installed jq")
    output.warning("careful")
    output.error("failed")
    output.debug("details")
    output.print("plain")
    output.hint("retry")

    #expect(stderr == [
        "==> Installing jq",
        "✓ Installed jq",
        "Warning: careful",
        "Error: failed",
        "DEBUG: details",
    ])
    #expect(stdout == [
        "plain",
        "Hint: retry",
    ])
}

@Test func quietSuppressesNonErrorOutput() {
    var stdout: [String] = []
    var stderr: [String] = []
    let output = Output(
        quiet: true,
        verbose: true,
        stdout: { stdout.append($0) },
        stderr: { stderr.append($0) }
    )

    output.info("hidden")
    output.success("hidden")
    output.debug("hidden")
    output.print("hidden")
    output.hint("hidden")
    output.warning("shown")
    output.error("shown")

    #expect(stdout.isEmpty)
    #expect(stderr == [
        "Warning: shown",
        "Error: shown",
    ])
}

@Test func tableAndPackageFormattingMatchPlainRustText() {
    var stdout: [String] = []
    let output = Output(stdout: { stdout.append($0) }, stderr: { _ in })

    output.tableHeader([("Name", 6), ("Version", 7)])
    output.tableRow([("jq", 6), ("1.7", 7)])
    output.packageInfo(name: "jq", version: "1.7", description: "JSON processor")
    output.listItem("jq", details: "1.7")
    output.section("Installed")
    output.caveats("first\nsecond")

    #expect(stdout == [
        "Name    Version",
        "---------------",
        "jq      1.7    ",
        "jq 1.7",
        "  JSON processor",
        "  • jq 1.7",
        "",
        "Installed",
        "",
        "==> Caveats",
        "  first",
        "  second",
    ])
}

@Test func humanFormattingMatchesRustHelpers() {
    #expect(formatDuration(59) == "59s")
    #expect(formatDuration(61) == "1m 1s")
    #expect(formatDuration(3_661) == "1h 1m")

    #expect(formatBytes(500) == "500 bytes")
    #expect(formatBytes(1_536) == "1.50 KB")
    #expect(formatBytes(1_572_864) == "1.50 MB")
    #expect(formatBytes(1_610_612_736) == "1.50 GB")
}
