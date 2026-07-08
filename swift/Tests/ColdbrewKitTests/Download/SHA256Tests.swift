import Foundation
import Testing
@testable import ColdbrewKit

@Test func sha256MatchesRustKnownDigest() throws {
    let data = Data("test content".utf8)
    let expected = "6ae8a75555209fd6c44157c0aed8016e763ff435a19cf186f76863140143ff72"

    #expect(SHA256.hash(data) == expected)

    let file = temporaryDirectory().appendingPathComponent("payload")
    try data.write(to: file)
    #expect(try SHA256.hash(file: file) == expected)
    #expect(try SHA256.verify(file: file, expected: expected))
    #expect(try !SHA256.verify(file: file, expected: "wrong_hash"))
}

@Test func sha256VerifyBottleReportsChecksumMismatch() throws {
    let file = temporaryDirectory().appendingPathComponent("payload")
    try Data("test content".utf8).write(to: file)

    do {
        try SHA256.verifyBottle(file: file, expected: "wrong_hash", package: "hello")
        Issue.record("expected checksum mismatch")
    } catch let error as ColdbrewError {
        #expect(error.description.contains("Checksum mismatch for 'hello'"))
    }
}
