import Foundation

public enum ColdbrewError: Error, CustomStringConvertible, LocalizedError, Equatable {
    case packageNotFound(String)
    case noBottleAvailable(package: String, platform: String)
    case checksumMismatch(package: String, expected: String, actual: String)
    case packageNotInstalled(name: String, version: String)
    case packageAlreadyInstalled(name: String, version: String)
    case versionNotAvailable(name: String, requested: String, available: String)
    case dependencyResolutionFailed(package: String, dependency: String)
    case circularDependency(String)
    case invalidVersion(String)
    case unsupportedPlatform(os: String, arch: String)
    case configError(String)
    case tapNotFound(String)
    case tapAlreadyExists(String)
    case invalidTapFormat(String)
    case lockfileNotFound
    case lockfileOutOfSync
    case projectNotFound
    case pathNotFound(String)
    case permissionDenied(String)
    case directoryCreationFailed(String)
    case extractionFailed(String)
    case cacheCorrupted(String)
    case indexNotInitialized
    case indexStale
    case packagePinned(String)
    case noDefaultVersion(String)
    case ghcrAuthFailed(String)
    case downloadFailed(String)
    case network(String)
    case io(String)
    case json(String)
    case toml(String)
    case git(String)
    case dialoguer(String)
    case walkdir(String)
    case database(String)
    case other(String)

    public var description: String {
        switch self {
        case .packageNotFound(let package):
            "Package '\(package)' not found"
        case .noBottleAvailable(let package, let platform):
            "No bottle available for '\(package)' on \(platform)"
        case .checksumMismatch(let package, let expected, let actual):
            "Checksum mismatch for '\(package)': expected \(expected), got \(actual)"
        case .packageNotInstalled(let name, let version):
            "Package '\(name)' version '\(version)' is not installed"
        case .packageAlreadyInstalled(let name, let version):
            "Package '\(name)' is already installed at version '\(version)'"
        case .versionNotAvailable(let name, let requested, let available):
            "Requested version '\(requested)' for '\(name)' is not available (current: \(available))"
        case .dependencyResolutionFailed(let package, let dependency):
            "Dependency '\(dependency)' required by '\(package)' could not be resolved"
        case .circularDependency(let chain):
            "Circular dependency detected: \(chain)"
        case .invalidVersion(let version):
            "Invalid version specification: '\(version)'"
        case .unsupportedPlatform(let os, let arch):
            "Unsupported platform: \(os) \(arch)"
        case .configError(let message):
            "Configuration error: \(message)"
        case .tapNotFound(let tap):
            "Tap '\(tap)' not found"
        case .tapAlreadyExists(let tap):
            "Tap '\(tap)' already exists"
        case .invalidTapFormat(let tap):
            "Invalid tap format: '\(tap)'. Expected 'user/repo'"
        case .lockfileNotFound:
            "Lockfile not found. Run 'crew lock' first"
        case .lockfileOutOfSync:
            "Lockfile is out of sync with coldbrew.toml. Run 'crew lock' to update"
        case .projectNotFound:
            "Project file not found. Run 'crew init' first"
        case .pathNotFound(let path):
            "Path not found: \(path)"
        case .permissionDenied(let path):
            "Permission denied: \(path)"
        case .directoryCreationFailed(let path):
            "Failed to create directory: \(path)"
        case .extractionFailed(let message):
            "Failed to extract archive: \(message)"
        case .cacheCorrupted(let message):
            "Cache is corrupted: \(message)"
        case .indexNotInitialized:
            "Index is not initialized. Run 'crew update' first"
        case .indexStale:
            "Index is stale. Run 'crew update' to refresh"
        case .packagePinned(let package):
            "Package '\(package)' is pinned and cannot be upgraded"
        case .noDefaultVersion(let package):
            "No default version set for '\(package)'"
        case .ghcrAuthFailed(let message):
            "GHCR authentication failed: \(message)"
        case .downloadFailed(let message):
            "Download failed: \(message)"
        case .network(let message):
            "Network error: \(message)"
        case .io(let message):
            "IO error: \(message)"
        case .json(let message):
            "JSON parsing error: \(message)"
        case .toml(let message):
            "TOML parsing error: \(message)"
        case .git(let message):
            "Git error: \(message)"
        case .dialoguer(let message):
            "Dialoguer error: \(message)"
        case .walkdir(let message):
            "Walkdir error: \(message)"
        case .database(let message):
            "Database error: \(message)"
        case .other(let message):
            message
        }
    }

    public var errorDescription: String? {
        description
    }

    public var suggestion: String? {
        switch self {
        case .packageNotFound:
            "Try 'crew search <term>' to find available packages"
        case .indexNotInitialized, .indexStale:
            "Run 'crew update' to fetch the latest package index"
        case .lockfileNotFound:
            "Run 'crew lock' to create a lockfile from coldbrew.toml"
        case .lockfileOutOfSync:
            "Run 'crew lock' to regenerate the lockfile from coldbrew.toml"
        case .projectNotFound:
            "Run 'crew init' to create a coldbrew.toml in this directory"
        case .noBottleAvailable:
            "This package may require building from source, which is not yet supported"
        case .packagePinned:
            "Use 'crew unpin <package>' to allow upgrades"
        case .checksumMismatch:
            "Try running 'crew clean' and retry the installation"
        case .versionNotAvailable:
            "Run 'crew info <package>' to see the current available version"
        default:
            nil
        }
    }

    public var isRetryable: Bool {
        switch self {
        case .network, .downloadFailed, .ghcrAuthFailed:
            true
        default:
            false
        }
    }
}

public typealias ColdbrewResult<Success> = Result<Success, ColdbrewError>
