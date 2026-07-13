import Foundation

public struct ResolvedDependency: Equatable, Sendable {
    public var name: String
    public var version: String
    public var isDirect: Bool
    public var depth: Int
}

public struct DependencyTree: Equatable, Sendable {
    public var name: String
    public var version: String
    public var children: [DependencyTree]

    public var totalCount: Int {
        children.reduce(0) { $0 + 1 + $1.totalCount }
    }
}

public final class DependencyResolver {
    private var formulas: [String: Formula] = [:]

    public init() {}

    public func addFormula(_ formula: Formula) {
        formulas[formula.name] = formula
    }

    public func addFormulas(_ formulas: some Sequence<Formula>) {
        for formula in formulas {
            addFormula(formula)
        }
    }

    public func resolve(_ packageName: String) throws -> [ResolvedDependency] {
        var resolved: [ResolvedDependency] = []
        var visited = Set<String>()
        var inProgress = Set<String>()
        try resolveRecursive(
            packageName,
            depth: 0,
            isDirect: true,
            resolved: &resolved,
            visited: &visited,
            inProgress: &inProgress
        )
        return resolved
    }

    public func dependencies(for packageName: String) throws -> [String] {
        guard let formula = formulas[packageName] else {
            throw CoreError.packageNotFound(packageName)
        }
        return formula.dependencies
    }

    public func dependents(of packageName: String) -> [String] {
        formulas
            .filter { $0.value.dependencies.contains(packageName) }
            .map(\.key)
            .sorted()
    }

    private func resolveRecursive(
        _ packageName: String,
        depth: Int,
        isDirect: Bool,
        resolved: inout [ResolvedDependency],
        visited: inout Set<String>,
        inProgress: inout Set<String>
    ) throws {
        if inProgress.contains(packageName) {
            throw CoreError.circularDependency(packageName)
        }
        if visited.contains(packageName) {
            return
        }
        guard let formula = formulas[packageName] else {
            throw CoreError.packageNotFound(packageName)
        }

        inProgress.insert(packageName)
        for dependency in formula.dependencies {
            try resolveRecursive(
                dependency,
                depth: depth + 1,
                isDirect: false,
                resolved: &resolved,
                visited: &visited,
                inProgress: &inProgress
            )
        }

        inProgress.remove(packageName)
        visited.insert(packageName)
        resolved.append(ResolvedDependency(
            name: packageName,
            version: formula.versions.stable,
            isDirect: isDirect,
            depth: depth
        ))
    }
}
