import Foundation

public let formulaIndexURL = URL(string: "https://formulae.brew.sh/api/formula.json")!
private let formulaAPIBaseEnv = "COLDBREW_FORMULA_API_BASE"

public struct HomebrewAPI: Sendable {
    public var baseURL: URL

    public init(baseURL: URL? = nil) {
        self.baseURL = baseURL ?? Self.defaultBaseURL()
    }

    public func fetchFormulaIndex() throws -> [Formula] {
        try decode([Formula].self, from: baseURL.appendingPathComponent("formula.json"))
    }

    public func fetchFormula(named name: String) throws -> Formula {
        let safeName = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        let url = baseURL
            .appendingPathComponent("formula", isDirectory: true)
            .appendingPathComponent("\(safeName).json")

        if url.isFileURL && !FileManager.default.fileExists(atPath: url.path) {
            throw ColdbrewError.packageNotFound(name)
        }
        return try decode(Formula.self, from: url)
    }

    private func decode<Value: Decodable>(_ type: Value.Type, from url: URL) throws -> Value {
        do {
            let data = try Data(contentsOf: url)
            return try JSONDecoder().decode(type, from: data)
        } catch let error as DecodingError {
            throw ColdbrewError.json(String(describing: error))
        } catch let error as ColdbrewError {
            throw error
        } catch {
            throw ColdbrewError.io(error.localizedDescription)
        }
    }

    private static func defaultBaseURL() -> URL {
        if let override = ProcessInfo.processInfo.environment[formulaAPIBaseEnv], !override.isEmpty {
            return URL(string: override) ?? URL(fileURLWithPath: override)
        }
        return URL(string: "https://formulae.brew.sh/api")!
    }
}
