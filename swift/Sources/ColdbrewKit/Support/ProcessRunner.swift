import Foundation

enum ProcessRunner {
    @discardableResult
    static func run(_ executable: String, _ arguments: [String]) throws -> Data {
        try output(executable, arguments)
    }

    static func output(_ executable: String, _ arguments: [String]) throws -> Data {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = [executable] + arguments

        let stdout = Pipe()
        let stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr

        do {
            try process.run()
        } catch {
            throw ColdbrewError.other("Failed to run '\(executable)': \(error.localizedDescription)")
        }

        process.waitUntilExit()
        let out = stdout.fileHandleForReading.readDataToEndOfFile()
        let err = stderr.fileHandleForReading.readDataToEndOfFile()
        guard process.terminationStatus == 0 else {
            let message = String(decoding: err, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
            throw ColdbrewError.other("'\(executable)' failed: \(message)")
        }
        return out
    }
}
