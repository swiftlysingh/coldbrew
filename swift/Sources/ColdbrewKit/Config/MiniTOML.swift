import Foundation

enum MiniTOML {
    typealias Table = [String: Value]

    enum Value: Equatable {
        case string(String)
        case bool(Bool)
        case integer(Int)
        case array([String])
        case inlineTable(Table)
    }

    static func parse(_ text: String) throws -> [String: Table] {
        var document: [String: Table] = ["": [:]]
        var section = ""

        for rawLine in text.split(whereSeparator: \.isNewline) {
            let line = stripComment(String(rawLine)).trimmingCharacters(in: .whitespacesAndNewlines)
            if line.isEmpty { continue }

            if line.hasPrefix("[") && line.hasSuffix("]") {
                section = String(line.dropFirst().dropLast())
                document[section, default: [:]] = document[section, default: [:]]
                continue
            }

            guard let equals = line.firstIndex(of: "=") else {
                throw ConfigError.invalidTOML("Missing '=' in line: \(line)")
            }

            let key = line[..<equals].trimmingCharacters(in: .whitespaces)
            let value = line[line.index(after: equals)...].trimmingCharacters(in: .whitespaces)
            document[section, default: [:]][key] = try parseValue(String(value))
        }

        return document
    }

    static func render(_ document: [String: Table], sectionOrder: [String]) -> String {
        var lines: [String] = []

        if let root = document[""] {
            for key in root.keys.sorted() {
                if let value = root[key] {
                    lines.append("\(key) = \(renderValue(value))")
                }
            }
        }

        for section in sectionOrder where section != "" {
            guard let table = document[section], !table.isEmpty else { continue }
            if !lines.isEmpty { lines.append("") }
            lines.append("[\(section)]")
            for key in table.keys.sorted() {
                if let value = table[key] {
                    lines.append("\(key) = \(renderValue(value))")
                }
            }
        }

        return lines.joined(separator: "\n") + "\n"
    }

    private static func stripComment(_ line: String) -> String {
        var inString = false
        var result = ""

        for character in line {
            if character == "\"" {
                inString.toggle()
            }
            if character == "#", !inString {
                break
            }
            result.append(character)
        }

        return result
    }

    private static func parseValue(_ value: String) throws -> Value {
        if value.hasPrefix("\""), value.hasSuffix("\"") {
            return .string(String(value.dropFirst().dropLast()))
        }
        if value == "true" { return .bool(true) }
        if value == "false" { return .bool(false) }
        if value.hasPrefix("["), value.hasSuffix("]") {
            let body = value.dropFirst().dropLast().trimmingCharacters(in: .whitespaces)
            if body.isEmpty { return .array([]) }
            return .array(splitCommaSeparated(String(body)).map { item in
                item.trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "\""))
            })
        }
        if value.hasPrefix("{"), value.hasSuffix("}") {
            let body = value.dropFirst().dropLast()
            var table: Table = [:]
            for part in splitCommaSeparated(String(body)) {
                guard let equals = part.firstIndex(of: "=") else {
                    throw ConfigError.invalidTOML("Invalid inline table entry: \(part)")
                }
                let key = part[..<equals].trimmingCharacters(in: .whitespaces)
                let raw = part[part.index(after: equals)...].trimmingCharacters(in: .whitespaces)
                table[key] = try parseValue(String(raw))
            }
            return .inlineTable(table)
        }
        if let integer = Int(value) {
            return .integer(integer)
        }
        return .string(value)
    }

    private static func renderValue(_ value: Value) -> String {
        switch value {
        case .string(let string):
            return "\"\(string.replacingOccurrences(of: "\"", with: "\\\""))\""
        case .bool(let bool):
            return bool ? "true" : "false"
        case .integer(let integer):
            return "\(integer)"
        case .array(let values):
            return "[" + values.map { renderValue(.string($0)) }.joined(separator: ", ") + "]"
        case .inlineTable(let table):
            let parts = table.keys.sorted().compactMap { key -> String? in
                guard let value = table[key] else { return nil }
                return "\(key) = \(renderValue(value))"
            }
            return "{ " + parts.joined(separator: ", ") + " }"
        }
    }

    private static func splitCommaSeparated(_ text: String) -> [String] {
        var parts: [String] = []
        var current = ""
        var inString = false

        for character in text {
            if character == "\"" {
                inString.toggle()
            }
            if character == ",", !inString {
                parts.append(current)
                current = ""
            } else {
                current.append(character)
            }
        }

        if !current.isEmpty {
            parts.append(current)
        }

        return parts
    }
}
