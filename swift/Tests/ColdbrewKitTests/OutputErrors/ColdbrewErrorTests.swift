import Testing
@testable import ColdbrewKit

@Test func errorDescriptionsMirrorRustMessages() {
    #expect(String(describing: ColdbrewError.packageNotFound("jq")) == "Package 'jq' not found")
    #expect(String(describing: ColdbrewError.noBottleAvailable(package: "jq", platform: "linux x86_64")) == "No bottle available for 'jq' on linux x86_64")
    #expect(String(describing: ColdbrewError.checksumMismatch(package: "jq", expected: "abc", actual: "def")) == "Checksum mismatch for 'jq': expected abc, got def")
    #expect(String(describing: ColdbrewError.packageNotInstalled(name: "jq", version: "1.7")) == "Package 'jq' version '1.7' is not installed")
    #expect(String(describing: ColdbrewError.packageAlreadyInstalled(name: "jq", version: "1.7")) == "Package 'jq' is already installed at version '1.7'")
    #expect(String(describing: ColdbrewError.versionNotAvailable(name: "jq", requested: "1.6", available: "1.7")) == "Requested version '1.6' for 'jq' is not available (current: 1.7)")
    #expect(String(describing: ColdbrewError.invalidTapFormat("bad")) == "Invalid tap format: 'bad'. Expected 'user/repo'")
    #expect(String(describing: ColdbrewError.lockfileOutOfSync) == "Lockfile is out of sync with coldbrew.toml. Run 'crew lock' to update")
    #expect(String(describing: ColdbrewError.indexStale) == "Index is stale. Run 'crew update' to refresh")
    #expect(String(describing: ColdbrewError.other("custom")) == "custom")
}

@Test func suggestionsMirrorRustMessages() {
    #expect(ColdbrewError.packageNotFound("jq").suggestion == "Try 'crew search <term>' to find available packages")
    #expect(ColdbrewError.indexNotInitialized.suggestion == "Run 'crew update' to fetch the latest package index")
    #expect(ColdbrewError.indexStale.suggestion == "Run 'crew update' to fetch the latest package index")
    #expect(ColdbrewError.lockfileNotFound.suggestion == "Run 'crew lock' to create a lockfile from coldbrew.toml")
    #expect(ColdbrewError.projectNotFound.suggestion == "Run 'crew init' to create a coldbrew.toml in this directory")
    #expect(ColdbrewError.noBottleAvailable(package: "jq", platform: "linux").suggestion == "This package may require building from source, which is not yet supported")
    #expect(ColdbrewError.packagePinned("jq").suggestion == "Use 'crew unpin <package>' to allow upgrades")
    #expect(ColdbrewError.checksumMismatch(package: "jq", expected: "a", actual: "b").suggestion == "Try running 'crew clean' and retry the installation")
    #expect(ColdbrewError.versionNotAvailable(name: "jq", requested: "1.6", available: "1.7").suggestion == "Run 'crew info <package>' to see the current available version")
    #expect(ColdbrewError.other("custom").suggestion == nil)
}

@Test func retryableErrorsMirrorRustPredicate() {
    #expect(ColdbrewError.network("offline").isRetryable)
    #expect(ColdbrewError.downloadFailed("timeout").isRetryable)
    #expect(ColdbrewError.ghcrAuthFailed("denied").isRetryable)
    #expect(!ColdbrewError.packageNotFound("jq").isRetryable)
}
