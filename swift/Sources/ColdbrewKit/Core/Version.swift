import Foundation

public struct Version: Codable, Comparable, CustomStringConvertible, Sendable {
    public let original: String
    private let components: [VersionComponent]

    public init(_ string: String) throws {
        guard !string.isEmpty else {
            throw CoreError.invalidVersion(string)
        }
        self.original = string
        self.components = Self.parseComponents(string)
    }

    public var description: String { original }

    public var isPrerelease: Bool {
        components.contains {
            if case let .alpha(value) = $0 {
                return value.contains("alpha")
                    || value.contains("beta")
                    || value.contains("rc")
                    || value.contains("pre")
                    || value.contains("dev")
            }
            return false
        }
    }

    public var major: UInt64? { numericComponents.first }
    public var minor: UInt64? { numericComponents.dropFirst().first }
    public var patch: UInt64? { numericComponents.dropFirst(2).first }

    private var numericComponents: [UInt64] {
        components.compactMap {
            if case let .numeric(value) = $0 { value } else { nil }
        }
    }

    public static func < (lhs: Version, rhs: Version) -> Bool {
        let lhsComponents = lhs.components.filter { $0 != .separator }
        let rhsComponents = rhs.components.filter { $0 != .separator }

        for (left, right) in zip(lhsComponents, rhsComponents) {
            let comparison = left.compare(to: right)
            if comparison != .orderedSame {
                return comparison == .orderedAscending
            }
        }

        return lhsComponents.count < rhsComponents.count
    }

    private static func parseComponents(_ string: String) -> [VersionComponent] {
        var components: [VersionComponent] = []
        var current = ""
        var inNumeric = false

        for character in string {
            if ".-_+".contains(character) {
                if !current.isEmpty {
                    components.append(makeComponent(current, isNumeric: inNumeric))
                    current = ""
                }
                components.append(.separator)
                inNumeric = false
            } else if character.isNumber {
                if !inNumeric && !current.isEmpty {
                    components.append(makeComponent(current, isNumeric: false))
                    current = ""
                }
                inNumeric = true
                current.append(character)
            } else {
                if inNumeric && !current.isEmpty {
                    components.append(makeComponent(current, isNumeric: true))
                    current = ""
                }
                inNumeric = false
                current.append(character)
            }
        }

        if !current.isEmpty {
            components.append(makeComponent(current, isNumeric: inNumeric))
        }

        return components
    }

    private static func makeComponent(_ string: String, isNumeric: Bool) -> VersionComponent {
        if isNumeric, let value = UInt64(string) {
            return .numeric(value)
        }
        return .alpha(string.lowercased())
    }
}

private enum VersionComponent: Codable, Equatable, Sendable {
    case numeric(UInt64)
    case alpha(String)
    case separator

    func compare(to other: VersionComponent) -> ComparisonResult {
        switch (self, other) {
        case let (.numeric(lhs), .numeric(rhs)):
            return lhs == rhs ? .orderedSame : (lhs < rhs ? .orderedAscending : .orderedDescending)
        case (.numeric, .alpha):
            return .orderedDescending
        case (.alpha, .numeric):
            return .orderedAscending
        case let (.alpha(lhs), .alpha(rhs)):
            return lhs == rhs ? .orderedSame : (lhs < rhs ? .orderedAscending : .orderedDescending)
        case (.separator, .separator):
            return .orderedSame
        case (.separator, _):
            return .orderedAscending
        case (_, .separator):
            return .orderedDescending
        }
    }
}

public func parsePackageSpec(_ spec: String) -> (name: String, version: String?) {
    guard let index = spec.firstIndex(of: "@") else {
        return (spec, nil)
    }
    return (
        String(spec[..<index]),
        String(spec[spec.index(after: index)...])
    )
}

public func versionMatches(_ version: Version, constraint: String) -> Bool {
    if version.original == constraint {
        return true
    }

    if let major = UInt64(constraint) {
        return version.major == major
    }

    let parts = constraint.split(separator: ".")
    if parts.count == 2,
       let major = UInt64(parts[0]),
       let minor = UInt64(parts[1]) {
        return version.major == major && version.minor == minor
    }

    return false
}

public enum CoreError: Error, Equatable, Sendable {
    case invalidVersion(String)
    case packageNotFound(String)
    case circularDependency(String)
}
