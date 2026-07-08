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
        switch (os, architecture) {
        case (.macOS, .arm64):
            return ["arm64_sequoia", "arm64_sonoma", "arm64_ventura"]
        case (.macOS, .x86_64):
            return ["sequoia", "sonoma", "ventura"]
        case (.linux, .x86_64):
            return ["x86_64_linux"]
        case (.linux, .arm64):
            return ["arm64_linux"]
        }
    }
}
