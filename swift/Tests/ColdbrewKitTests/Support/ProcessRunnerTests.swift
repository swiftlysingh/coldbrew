import Testing
@testable import ColdbrewKit

@Test func processRunnerDrainsStdoutAndStderrWhileProcessRuns() throws {
    let result = try ProcessRunner.capture("sh", ["-c", "head -c 262144 /dev/zero; head -c 262144 /dev/zero >&2"])
    #expect(result.status == 0)
    #expect(result.stdout.count == 262_144)
    #expect(result.stderr.count == 262_144)
}
