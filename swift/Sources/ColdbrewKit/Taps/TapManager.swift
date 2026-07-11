import Foundation

public struct Tap: Equatable, Sendable {
    public var user: String
    public var repo: String
    public var path: URL

    public var fullName: String { "\(user)/\(repo)" }
    public var githubURL: String { "https://github.com/\(user)/\(repo).git" }
}

public struct TapManager: Sendable {
    public let paths: Paths

    public init(paths: Paths) {
        self.paths = paths
    }

    public static func parse(_ name: String) throws -> (user: String, repo: String) {
        let parts = name.split(separator: "/", omittingEmptySubsequences: false)
        guard parts.count == 2, !parts[0].isEmpty, !parts[1].isEmpty else {
            throw ColdbrewError.invalidTapFormat(name)
        }
        let repo = parts[1].hasPrefix("homebrew-") ? String(parts[1]) : "homebrew-\(parts[1])"
        return (String(parts[0]), repo)
    }

    @discardableResult
    public func add(_ name: String) throws -> Tap {
        let (user, repo) = try Self.parse(name)
        let path = paths.tapDir(user: user, repo: repo)
        guard !FileManager.default.fileExists(atPath: path.path) else {
            throw ColdbrewError.tapAlreadyExists("\(user)/\(repo)")
        }
        try FileManager.default.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
        try runGit(["clone", "https://github.com/\(user)/\(repo).git", path.path])
        return Tap(user: user, repo: repo, path: path)
    }

    public func remove(_ name: String) throws {
        let (user, repo) = try Self.parse(name)
        let path = paths.tapDir(user: user, repo: repo)
        guard FileManager.default.fileExists(atPath: path.path) else {
            throw ColdbrewError.tapNotFound("\(user)/\(repo)")
        }
        try FileManager.default.removeItem(at: path)
        let userDir = paths.tapsDir.appendingPathComponent(user, isDirectory: true)
        if (try? FileManager.default.contentsOfDirectory(atPath: userDir.path).isEmpty) == true {
            try FileManager.default.removeItem(at: userDir)
        }
    }

    public func list() throws -> [Tap] {
        guard FileManager.default.fileExists(atPath: paths.tapsDir.path) else {
            return []
        }
        let users = try FileManager.default.contentsOfDirectory(at: paths.tapsDir, includingPropertiesForKeys: [.isDirectoryKey])
        var taps: [Tap] = []
        for userURL in users where try userURL.resourceValues(forKeys: [.isDirectoryKey]).isDirectory == true {
            let repos = try FileManager.default.contentsOfDirectory(at: userURL, includingPropertiesForKeys: [.isDirectoryKey])
            for repoURL in repos where try repoURL.resourceValues(forKeys: [.isDirectoryKey]).isDirectory == true {
                taps.append(Tap(user: userURL.lastPathComponent, repo: repoURL.lastPathComponent, path: repoURL))
            }
        }
        return taps.sorted { $0.fullName < $1.fullName }
    }

    public func update(_ name: String) throws {
        let (user, repo) = try Self.parse(name)
        let path = paths.tapDir(user: user, repo: repo)
        guard FileManager.default.fileExists(atPath: path.path) else {
            throw ColdbrewError.tapNotFound("\(user)/\(repo)")
        }
        try runGit(["-C", path.path, "pull", "--ff-only"])
    }

    private func runGit(_ arguments: [String]) throws {
        do {
            let result = try ProcessRunner.capture("git", arguments)
            guard result.status == 0 else {
                let message = String(decoding: result.stderr, as: UTF8.self)
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                throw ColdbrewError.git(message)
            }
        } catch let error as ColdbrewError {
            if case .git = error { throw error }
            throw ColdbrewError.git(error.description)
        } catch {
            throw ColdbrewError.git(error.localizedDescription)
        }
    }
}
