import Testing
@testable import ColdbrewKit

@Test func dependencyResolverOrdersDependenciesBeforeRoot() throws {
    let dep = Formula(name: "dep", versions: Versions(stable: "1.0.0"))
    let root = Formula(name: "root", versions: Versions(stable: "2.0.0"), dependencies: ["dep"])
    let resolver = DependencyResolver()
    resolver.addFormulas([root, dep])

    let resolved = try resolver.resolve("root")

    #expect(resolved.map(\.name) == ["dep", "root"])
    #expect(resolved[0].isDirect == false)
    #expect(resolved[1].isDirect == true)
}

@Test func dependencyResolverDetectsCircularDependencies() throws {
    let a = Formula(name: "a", versions: Versions(stable: "1.0.0"), dependencies: ["b"])
    let b = Formula(name: "b", versions: Versions(stable: "1.0.0"), dependencies: ["a"])
    let resolver = DependencyResolver()
    resolver.addFormulas([a, b])

    #expect(throws: CoreError.circularDependency("a")) {
        try resolver.resolve("a")
    }
}
