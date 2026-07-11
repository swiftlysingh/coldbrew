import Foundation
import Testing

@Test func rootHelpListsPublicCommandsAndGlobalFlags() throws {
    let output = try runCrew("--help")

    #expect(output.status == 0)
    #expect(output.text.contains("--quiet"))
    #expect(output.text.contains("--verbose"))

    for command in [
        "update",
        "search",
        "info",
        "install",
        "uninstall",
        "upgrade",
        "list",
        "which",
        "pin",
        "unpin",
        "default",
        "dependents",
        "init",
        "lock",
        "tap",
        "space",
        "link",
        "unlink",
        "doctor",
        "completions",
    ] {
        #expect(commandIsListed(command, in: output.text))
    }

    #expect(!commandIsListed("exec", in: output.text))
}

@Test func commandHelpExposesCurrentFlagsAndArguments() throws {
    let cases: [([String], [String])] = [
        (["install", "--help"], ["packages", "--lock", "--skip-deps", "--force"]),
        (["uninstall", "--help"], ["packages", "--all", "--with-deps"]),
        (["upgrade", "--help"], ["packages", "--yes"]),
        (["list", "--help"], ["--names-only", "--versions"]),
        (["which", "--help"], ["binary"]),
        (["pin", "--help"], ["package"]),
        (["unpin", "--help"], ["package"]),
        (["default", "--help"], ["package"]),
        (["dependents", "--help"], ["package"]),
        (["init", "--help"], ["--force"]),
        (["lock", "--help"], ["USAGE:"]),
        (["tap", "--help"], ["tap", "--remove"]),
        (["space", "--help"], ["show", "clean"]),
        (["space", "show", "--help"], ["--details"]),
        (["space", "clean", "--help"], ["--all", "--dry-run"]),
        (["link", "--help"], ["package", "--force"]),
        (["unlink", "--help"], ["package"]),
        (["doctor", "--help"], ["USAGE:"]),
        (["completions", "--help"], ["shell"]),
    ]

    for (args, expectedTokens) in cases {
        let output = try runCrew(args)

        #expect(output.status == 0)
        for token in expectedTokens {
            #expect(
                output.text.localizedCaseInsensitiveContains(token),
                "Expected \(args.joined(separator: " ")) help to contain \(token). Output:\n\(output.text)"
            )
        }
    }
}

@Test func hiddenExecCommandIsCallableForShims() throws {
    let output = try runCrew("exec", "--help")

    #expect(output.status == 0)
    for token in ["package", "binary", "args"] {
        #expect(output.text.localizedCaseInsensitiveContains(token))
    }
}

@Test func completionsCommandUsesArgumentParserGenerator() throws {
    let output = try runCrew("completions", "zsh")

    #expect(output.status == 0)
    #expect(output.text.contains("#compdef crew"))
}

@Test func globalQuietAndVerboseFlagsAffectSubcommands() throws {
    let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    defer { try? FileManager.default.removeItem(at: home) }
    let environment = ["COLDBREW_HOME": home.path]

    let quiet = try runCrew(["--quiet", "list"], environment: environment)
    #expect(quiet.status == 0)
    #expect(quiet.text.isEmpty)

    let verbose = try runCrew(["list", "--verbose"], environment: environment)
    #expect(verbose.status == 0)
    #expect(verbose.text.contains("DEBUG: Running list"))
}

@Test func installLockDoesNotRequirePackageArguments() throws {
    let output = try runCrew("install", "--lock")

    #expect(output.status != 0)
    #expect(output.text.contains("Lockfile not found"))
    #expect(!output.text.contains("Missing expected argument"))
}

private struct CrewOutput {
    let status: Int32
    let text: String
}

private func runCrew(_ args: String...) throws -> CrewOutput {
    try runCrew(args)
}

private func runCrew(_ args: [String], environment: [String: String] = [:]) throws -> CrewOutput {
    guard let crew = crewBinary() else {
        Issue.record("Missing built crew executable; run `swift build` before `swift test`.")
        return CrewOutput(status: 127, text: "")
    }

    let process = Process()
    process.executableURL = crew
    process.arguments = args
    process.environment = ProcessInfo.processInfo.environment.merging(environment) { _, override in override }

    let stdout = Pipe()
    let stderr = Pipe()
    process.standardOutput = stdout
    process.standardError = stderr

    try process.run()
    process.waitUntilExit()

    let stdoutText = String(decoding: stdout.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
    let stderrText = String(decoding: stderr.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)

    return CrewOutput(status: process.terminationStatus, text: stdoutText + stderrText)
}

private func crewBinary() -> URL? {
    let packageRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()

    for relativePath in [
        ".build/debug/crew",
        ".build/arm64-apple-macosx/debug/crew",
        ".build/x86_64-apple-macosx/debug/crew",
        ".build/aarch64-unknown-linux-gnu/debug/crew",
        ".build/x86_64-unknown-linux-gnu/debug/crew",
    ] {
        let url = packageRoot.appendingPathComponent(relativePath)
        if FileManager.default.isExecutableFile(atPath: url.path) {
            return url
        }
    }

    return nil
}

private func commandIsListed(_ command: String, in help: String) -> Bool {
    help.split(separator: "\n").contains { line in
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        return trimmed == command || trimmed.hasPrefix(command + " ")
    }
}
