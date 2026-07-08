import Foundation

public struct UpgradeInfo: Equatable, Sendable {
    public var name: String
    public var currentVersion: String
    public var newVersion: String
    public var isMajor: Bool
}

public struct UpgradePlan: Equatable, Sendable {
    public var upgrades: [UpgradeInfo]
}

public struct UpgradeResult: Equatable, Sendable {
    public var upgraded: [UpgradeInfo]
}

public struct UpgradeManager: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    public func plan(available: [InstallRequest], filter: [String] = []) throws -> UpgradePlan {
        let availableByName = Dictionary(uniqueKeysWithValues: available.map { ($0.name, $0) })
        let config = try SimpleGlobalConfig.load(paths: paths)
        let installed = try Cellar(paths: paths).listPackages()
        let filterSet = Set(filter)
        let upgrades = installed.compactMap { package -> UpgradeInfo? in
            guard filterSet.isEmpty || filterSet.contains(package.name) else { return nil }
            guard config.pins[package.name] == nil else { return nil }
            guard let candidate = availableByName[package.name], candidate.version != package.version else { return nil }
            return UpgradeInfo(
                name: package.name,
                currentVersion: package.version,
                newVersion: candidate.version,
                isMajor: major(package.version) != major(candidate.version)
            )
        }.sorted { $0.name < $1.name }
        return UpgradePlan(upgrades: upgrades)
    }

    public func apply(_ plan: UpgradePlan, available: [InstallRequest], yes: Bool) async throws -> UpgradeResult {
        guard yes else {
            return UpgradeResult(upgraded: [])
        }

        let requests = Dictionary(uniqueKeysWithValues: available.map { ($0.name, $0) })
        var upgraded: [UpgradeInfo] = []
        for upgrade in plan.upgrades {
            guard let request = requests[upgrade.name] else { continue }
            _ = try await InstallManager(paths: paths).install([request], options: InstallOptions(force: true))
            try PackageOperations(paths: paths).setDefault("\(upgrade.name)@\(upgrade.newVersion)")
            upgraded.append(upgrade)
        }
        return UpgradeResult(upgraded: upgraded)
    }
}

private func major(_ version: String) -> Int? {
    let prefix = version.split(whereSeparator: { !$0.isNumber }).first
    return prefix.flatMap { Int($0) }
}
