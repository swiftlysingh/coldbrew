import Foundation

enum ProcessRunner {
    @discardableResult
    static func run(_ executable: String, _ arguments: [String]) throws -> Data {
        try output(executable, arguments)
    }

    static func output(_ executable: String, _ arguments: [String]) throws -> Data {
        let result = try capture(executable, arguments)
        guard result.status == 0 else {
            let message = String(decoding: result.stderr, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
            throw ColdbrewError.other("'\(executable)' failed: \(message)")
        }
        return result.stdout
    }

    static func capture(_ executable: String, _ arguments: [String]) throws -> (status: Int32, stdout: Data, stderr: Data) {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [executable] + arguments

        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let stdoutURL = root.appendingPathExtension("stdout")
        let stderrURL = root.appendingPathExtension("stderr")
        FileManager.default.createFile(atPath: stdoutURL.path, contents: nil)
        FileManager.default.createFile(atPath: stderrURL.path, contents: nil)
        let stdout = try FileHandle(forWritingTo: stdoutURL)
        let stderr = try FileHandle(forWritingTo: stderrURL)
        defer {
            try? stdout.close()
            try? stderr.close()
            try? FileManager.default.removeItem(at: stdoutURL)
            try? FileManager.default.removeItem(at: stderrURL)
        }
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
        } catch {
            throw ColdbrewError.other("Failed to run '\(executable)': \(error.localizedDescription)")
        }

        process.waitUntilExit()
        try stdout.close()
        try stderr.close()
        return (process.terminationStatus, try Data(contentsOf: stdoutURL), try Data(contentsOf: stderrURL))
    }
}
