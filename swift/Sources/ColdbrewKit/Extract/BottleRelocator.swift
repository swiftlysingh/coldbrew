import Foundation

public struct RelocationSummary: Equatable, Sendable {
    public var scannedFiles = 0
    public var machOFiles = 0
    public var relocatedFiles = 0
    public var unhandledPlaceholders = 0
}

public struct BottleRelocator: Sendable {
    private let root: URL
    private let cellar: URL

    public init(paths: Paths) {
        self.root = paths.root
        self.cellar = paths.cellarDir
    }

    public func relocateBottle(at installPath: URL) throws -> RelocationSummary {
        var summary = RelocationSummary()
        #if os(macOS)
        let files = try regularFiles(under: installPath)
        for file in files {
            summary.scannedFiles += 1
            guard try isMachO(file) else { continue }
            summary.machOFiles += 1
            guard try containsHomebrewPlaceholders(file) else { continue }
            if try relocateMachO(file) {
                summary.relocatedFiles += 1
            } else {
                summary.unhandledPlaceholders += 1
            }
        }
        #endif
        return summary
    }

    public func codesignMachOTree(at installPath: URL) throws -> Int {
        var signed = 0
        #if os(macOS)
        for file in try regularFiles(under: installPath) where try isMachO(file) {
            try ensureWritable(file)
            try ProcessRunner.run("codesign", ["--sign", "-", "--force", "--timestamp=none", file.path])
            signed += 1
        }
        #endif
        return signed
    }

    private func relocateMachO(_ file: URL) throws -> Bool {
        let commands = try loadCommandPaths(file)
        var args: [String] = []
        for command in commands {
            guard let replaced = replacePlaceholders(in: command.value) else { continue }
            switch command.kind {
            case .rpath:
                args += ["-rpath", command.value, replaced]
            case .idDylib:
                args += ["-id", replaced]
            case .loadDylib:
                args += ["-change", command.value, replaced]
            }
        }
        guard !args.isEmpty else {
            return false
        }
        try ensureWritable(file)
        args.append(file.path)
        try ProcessRunner.run("install_name_tool", args)
        return true
    }

    private func loadCommandPaths(_ file: URL) throws -> [LoadCommandPath] {
        let output = try ProcessRunner.output("otool", ["-l", file.path])
        let text = String(decoding: output, as: UTF8.self)
        var current: String?
        var commands: [LoadCommandPath] = []

        for line in text.split(separator: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if let cmd = trimmed.dropPrefix("cmd ") {
                current = String(cmd.trimmingCharacters(in: .whitespaces))
                continue
            }
            guard let current else { continue }
            if current == "LC_RPATH", let value = parseLoadValue(trimmed, prefix: "path ") {
                commands.append(LoadCommandPath(kind: .rpath, value: value))
            } else if current.hasPrefix("LC_"), current.contains("DYLIB"), let value = parseLoadValue(trimmed, prefix: "name ") {
                commands.append(LoadCommandPath(kind: current == "LC_ID_DYLIB" ? .idDylib : .loadDylib, value: value))
            }
        }
        return commands
    }

    private func replacePlaceholders(in value: String) -> String? {
        let replaced = value
            .replacingOccurrences(of: "@@HOMEBREW_CELLAR@@", with: cellar.path)
            .replacingOccurrences(of: "@@HOMEBREW_PREFIX@@", with: root.path)
        return replaced == value ? nil : replaced
    }
}

private enum LoadCommandKind {
    case rpath
    case idDylib
    case loadDylib
}

private struct LoadCommandPath {
    var kind: LoadCommandKind
    var value: String
}

private func regularFiles(under root: URL) throws -> [URL] {
    guard let enumerator = FileManager.default.enumerator(at: root, includingPropertiesForKeys: [.isRegularFileKey]) else {
        return []
    }
    var files: [URL] = []
    for case let file as URL in enumerator {
        if try file.resourceValues(forKeys: [.isRegularFileKey]).isRegularFile == true {
            files.append(file)
        }
    }
    return files
}

private func isMachO(_ file: URL) throws -> Bool {
    let handle = try FileHandle(forReadingFrom: file)
    defer { try? handle.close() }
    let bytes = [UInt8](handle.readData(ofLength: 4))
    guard bytes.count == 4 else { return false }
    let be = UInt32(bytes[0]) << 24 | UInt32(bytes[1]) << 16 | UInt32(bytes[2]) << 8 | UInt32(bytes[3])
    let le = UInt32(bytes[3]) << 24 | UInt32(bytes[2]) << 16 | UInt32(bytes[1]) << 8 | UInt32(bytes[0])
    return [0xfeedface, 0xfeedfacf, 0xcafebabe, 0xcafebabf].contains(be)
        || [0xfeedface, 0xfeedfacf, 0xcafebabe, 0xcafebabf].contains(le)
}

private func containsHomebrewPlaceholders(_ file: URL) throws -> Bool {
    let data = try Data(contentsOf: file)
    return data.range(of: Data("@@HOMEBREW_CELLAR@@".utf8)) != nil
        || data.range(of: Data("@@HOMEBREW_PREFIX@@".utf8)) != nil
}

private func parseLoadValue(_ line: String, prefix: String) -> String? {
    guard let rest = line.dropPrefix(prefix) else { return nil }
    let value = String(rest)
    if let offset = value.range(of: " (offset ") {
        return String(value[..<offset.lowerBound]).trimmingCharacters(in: .whitespaces)
    }
    return value.trimmingCharacters(in: .whitespaces)
}

private func ensureWritable(_ file: URL) throws {
    let attrs = try FileManager.default.attributesOfItem(atPath: file.path)
    let mode = (attrs[.posixPermissions] as? NSNumber)?.uint16Value ?? 0o644
    if mode & 0o200 == 0 {
        try FileManager.default.setAttributes([.posixPermissions: mode | 0o200], ofItemAtPath: file.path)
    }
}

private extension String {
    func dropPrefix(_ prefix: String) -> Substring? {
        hasPrefix(prefix) ? dropFirst(prefix.count) : nil
    }
}
