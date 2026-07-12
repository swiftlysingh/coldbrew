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
        let availableByName = Dictionary(grouping: available, by: \.name)
        let config = try SimpleGlobalConfig.load(paths: paths)
        let installedByName = Dictionary(grouping: try Cellar(paths: paths).listPackages(), by: \.name)
        let filterSet = Set(filter)
        let upgrades = installedByName.compactMap { name, packages -> UpgradeInfo? in
            guard filterSet.isEmpty || filterSet.contains(name) else { return nil }
            guard config.pins[name] == nil else { return nil }
            guard let package = packages.max(by: { versionLessThan($0.version, $1.version) }),
                  let candidate = availableByName[name]?.max(by: { versionLessThan($0.version, $1.version) }),
                  versionLessThan(package.version, candidate.version)
            else { return nil }
            return UpgradeInfo(
                name: name,
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

        var upgraded: [UpgradeInfo] = []
        for upgrade in plan.upgrades {
            guard let request = available.first(where: { $0.name == upgrade.name && $0.version == upgrade.newVersion }) else { continue }
            _ = try await InstallManager(paths: paths).install([request], options: InstallOptions(force: true))
            try PackageOperations(paths: paths).setDefault("\(upgrade.name)@\(upgrade.newVersion)")
            upgraded.append(upgrade)
        }
        return UpgradeResult(upgraded: upgraded)
    }
}

private func versionLessThan(_ lhs: String, _ rhs: String) -> Bool {
    guard let left = try? Version(lhs), let right = try? Version(rhs) else { return lhs < rhs }
    return left < right
}

private func major(_ version: String) -> Int? {
    let prefix = version.split(whereSeparator: { !$0.isNumber }).first
    return prefix.flatMap { Int($0) }
}
