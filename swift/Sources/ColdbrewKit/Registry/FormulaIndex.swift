import Foundation

public final class FormulaIndex: @unchecked Sendable {
    private let indexURL: URL
    private var formulas: [String: Formula]?

    public init(paths: Paths) {
        self.indexURL = paths.formulaIndex
    }

    public init(indexURL: URL) {
        self.indexURL = indexURL
    }

    public var exists: Bool {
        FileManager.default.fileExists(atPath: indexURL.path)
    }

    @discardableResult
    public func update(from api: HomebrewAPI) throws -> Int {
        let formulas = try api.fetchFormulaIndex()
        try FileManager.default.createDirectory(
            at: indexURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        try encoder.encode(formulas).write(to: indexURL, options: .atomic)
        self.formulas = Self.mapByName(formulas)
        return formulas.count
    }

    public func load() throws {
        guard exists else {
            throw ColdbrewError.indexNotInitialized
        }

        do {
            let data = try Data(contentsOf: indexURL)
            let formulas = try JSONDecoder().decode([Formula].self, from: data)
            self.formulas = Self.mapByName(formulas)
        } catch let error as DecodingError {
            throw ColdbrewError.json(String(describing: error))
        } catch let error as ColdbrewError {
            throw error
        } catch {
            throw ColdbrewError.io(error.localizedDescription)
        }
    }

    public func formula(named name: String) throws -> Formula? {
        try loadIfNeeded()
        return formulas?[name]
    }

    public func search(_ query: String) throws -> [Formula] {
        try loadIfNeeded()
        let query = query.lowercased()
        return (formulas.map { Array($0.values) } ?? [])
            .filter {
                $0.name.lowercased().contains(query)
                    || ($0.desc?.lowercased().contains(query) ?? false)
            }
            .sorted { lhs, rhs in
                rank(lhs, query: query) == rank(rhs, query: query)
                    ? lhs.name < rhs.name
                    : rank(lhs, query: query) < rank(rhs, query: query)
            }
    }

    public func listFormulas() throws -> [Formula] {
        try loadIfNeeded()
        return (formulas.map { Array($0.values) } ?? []).sorted { $0.name < $1.name }
    }

    private func loadIfNeeded() throws {
        if formulas == nil {
            try load()
        }
    }

    private func rank(_ formula: Formula, query: String) -> Int {
        let name = formula.name.lowercased()
        if name == query {
            return 0
        }
        if name.hasPrefix(query) {
            return 1
        }
        if name.contains(query) {
            return 2
        }
        return 3
    }

    private static func mapByName(_ formulas: [Formula]) -> [String: Formula] {
        var map: [String: Formula] = [:]
        for formula in formulas {
            map[formula.name] = formula
        }
        return map
    }
}
