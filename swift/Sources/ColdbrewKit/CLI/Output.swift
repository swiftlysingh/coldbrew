import Foundation

public struct Output {
    public typealias Writer = (String) -> Void

    private let quiet: Bool
    private let verbose: Bool
    private let stdout: Writer
    private let stderr: Writer

    public init(
        quiet: Bool = false,
        verbose: Bool = false,
        stdout: @escaping Writer = { Swift.print($0) },
        stderr: @escaping Writer = { message in
            FileHandle.standardError.write(Data((message + "\n").utf8))
        }
    ) {
        self.quiet = quiet
        self.verbose = verbose
        self.stdout = stdout
        self.stderr = stderr
    }

    public func info(_ message: String) {
        guard !quiet else { return }
        stderr("==> \(message)")
    }

    public func success(_ message: String) {
        guard !quiet else { return }
        stderr("✓ \(message)")
    }

    public func warning(_ message: String) {
        stderr("Warning: \(message)")
    }

    public func error(_ message: String) {
        stderr("Error: \(message)")
    }

    public func debug(_ message: String) {
        guard verbose, !quiet else { return }
        stderr("DEBUG: \(message)")
    }

    public func print(_ message: String) {
        guard !quiet else { return }
        stdout(message)
    }

    public static func packageName(_ name: String) -> String {
        name
    }

    public static func version(_ version: String) -> String {
        version
    }

    public func tableHeader(_ columns: [(String, Int)]) {
        guard !quiet else { return }
        let header = tableLine(columns)
        stdout(header)
        stdout(String(repeating: "-", count: header.count))
    }

    public func tableRow(_ values: [(String, Int)]) {
        guard !quiet else { return }
        stdout(tableLine(values))
    }

    public func packageInfo(name: String, version: String, description: String?) {
        guard !quiet else { return }
        stdout("\(Self.packageName(name)) \(Self.version(version))")
        if let description {
            stdout("  \(description)")
        }
    }

    public func listItem(_ item: String, details: String? = nil) {
        guard !quiet else { return }
        stdout("  • \(item)" + details.map { " \($0)" }.orEmpty)
    }

    public func section(_ title: String) {
        guard !quiet else { return }
        stdout("")
        stdout(title)
    }

    public func caveats(_ caveats: String) {
        guard !quiet else { return }
        stdout("")
        stdout("==> Caveats")
        for line in caveats.split(separator: "\n", omittingEmptySubsequences: false) {
            stdout("  \(line)")
        }
    }

    public func hint(_ message: String) {
        guard !quiet else { return }
        stdout("Hint: \(message)")
    }

    private func tableLine(_ values: [(String, Int)]) -> String {
        values.map { value, width in
            value.padding(toLength: max(value.count, width), withPad: " ", startingAt: 0)
        }.joined(separator: "  ")
    }
}

public func formatDuration(_ seconds: UInt64) -> String {
    if seconds < 60 {
        return "\(seconds)s"
    } else if seconds < 3_600 {
        return "\(seconds / 60)m \(seconds % 60)s"
    } else {
        return "\(seconds / 3_600)h \((seconds % 3_600) / 60)m"
    }
}

public func formatBytes(_ bytes: UInt64) -> String {
    let kb: UInt64 = 1_024
    let mb = kb * 1_024
    let gb = mb * 1_024

    if bytes >= gb {
        return String(format: "%.2f GB", Double(bytes) / Double(gb))
    } else if bytes >= mb {
        return String(format: "%.2f MB", Double(bytes) / Double(mb))
    } else if bytes >= kb {
        return String(format: "%.2f KB", Double(bytes) / Double(kb))
    } else {
        return "\(bytes) bytes"
    }
}

private extension Optional where Wrapped == String {
    var orEmpty: String {
        self ?? ""
    }
}
