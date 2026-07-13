import Foundation

public struct Package: Codable, Equatable, Sendable {
    public var name: String
    public var version: String
    public var binaries: [String]
    public var dependencies: [String]

    public init(name: String, version: String, binaries: [String] = [], dependencies: [String] = []) {
        self.name = name
        self.version = version
        self.binaries = binaries
        self.dependencies = dependencies
    }
}
