import Foundation

public struct Platform: Equatable, Sendable {
    public enum OS: String, Sendable {
        case macOS
        case linux
    }

    public enum Architecture: String, Sendable {
        case arm64
        case x86_64
    }

    public var os: OS
    public var architecture: Architecture
    private var macOSMajorVersion: Int?

    public init(os: OS, architecture: Architecture, macOSMajorVersion: Int? = nil) {
        self.os = os
        self.architecture = architecture
        self.macOSMajorVersion = macOSMajorVersion
    }

    public var bottleTags: [String] {
        switch (os, architecture) {
        case (.macOS, .arm64):
            return macOSBottleTags(prefix: "arm64_")
        case (.macOS, .x86_64):
            return macOSBottleTags(prefix: "")
        case (.linux, .x86_64):
            return ["x86_64_linux", "all"]
        case (.linux, .arm64):
            return ["arm64_linux", "all"]
        }
    }

    private func macOSBottleTags(prefix: String) -> [String] {
        let majorVersion = macOSMajorVersion ?? ProcessInfo.processInfo.operatingSystemVersion.majorVersion
        let current = switch majorVersion {
        case 26: "tahoe"
        case 15: "sequoia"
        case 14: "sonoma"
        case 13: "ventura"
        case 12: "monterey"
        case 11: "big_sur"
        case 10: "catalina"
        default: "macos\(majorVersion)"
        }
        return ([current, "sequoia", "sonoma", "ventura"]
            .reduce(into: [String]()) { if !$0.contains($1) { $0.append($1) } }
            .map { prefix + $0 }) + ["all"]
    }
}
