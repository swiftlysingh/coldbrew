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

    public init(os: OS, architecture: Architecture) {
        self.os = os
        self.architecture = architecture
    }

    public var bottleTags: [String] {
        let macOSVersions: [String]
        #if os(macOS)
        let current = ProcessInfo.processInfo.operatingSystemVersion.majorVersion
        macOSVersions = [
            (current >= 15 ? "sequoia" : nil),
            (current >= 14 ? "sonoma" : nil),
            (current >= 13 ? "ventura" : nil),
        ].compactMap { $0 }
        #else
        macOSVersions = ["sequoia", "sonoma", "ventura"]
        #endif

        switch (os, architecture) {
        case (.macOS, .arm64):
            return macOSVersions.map { "arm64_\($0)" } + ["all"]
        case (.macOS, .x86_64):
            return macOSVersions + ["all"]
        case (.linux, .x86_64):
            return ["x86_64_linux"]
        case (.linux, .arm64):
            return ["arm64_linux"]
        }
    }
}
