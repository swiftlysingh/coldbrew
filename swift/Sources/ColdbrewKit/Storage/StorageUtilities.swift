import Foundation

#if canImport(Glibc)
import Glibc
#elseif canImport(Musl)
import Musl
#else
import Darwin
#endif

func directorySize(_ url: URL) throws -> UInt64 {
    guard FileManager.default.fileExists(atPath: url.path) else {
        throw ColdbrewError.pathNotFound(url.path)
    }

    var total: UInt64 = 0
    guard let enumerator = FileManager.default.enumerator(
        at: url,
        includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey],
        options: [.skipsHiddenFiles]
    ) else {
        return 0
    }

    for case let file as URL in enumerator {
        let values = try file.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        if values.isRegularFile == true {
            total += UInt64(values.fileSize ?? 0)
        }
    }
    return total
}

func copyTree(from source: URL, to destination: URL) throws {
    try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)

    let sourcePath = source.standardizedFileURL.path
    guard let enumerator = FileManager.default.enumerator(
        at: source,
        includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey]
    ) else {
        return
    }

    for case let item as URL in enumerator {
        let itemPath = item.standardizedFileURL.path
        guard itemPath.hasPrefix(sourcePath + "/") else {
            continue
        }
        let relativePath = String(itemPath.dropFirst(sourcePath.count + 1))

        let target = destination.appendingPathComponent(relativePath)
        let values = try item.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])
        if values.isSymbolicLink == true {
            try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
            try FileManager.default.createSymbolicLink(
                atPath: target.path,
                withDestinationPath: FileManager.default.destinationOfSymbolicLink(atPath: item.path)
            )
            continue
        }
        if values.isDirectory == true {
            try FileManager.default.createDirectory(at: target, withIntermediateDirectories: true)
            continue
        }

        try FileManager.default.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
        if FileManager.default.fileExists(atPath: target.path) {
            try FileManager.default.removeItem(at: target)
        }
        do {
            try FileManager.default.linkItem(at: item, to: target)
        } catch {
            try FileManager.default.copyItem(at: item, to: target)
        }
    }
}

func setExecutable(_ url: URL) throws {
    try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: url.path)
}

final class FileLock {
    private let path: URL
    private var ownsLock = false

    init(path: URL, timeout: TimeInterval = 30) throws {
        self.path = path
        try FileManager.default.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)

        let deadline = Date().addingTimeInterval(timeout)
        while true {
            let fd = open(path.path, O_CREAT | O_EXCL | O_WRONLY, 0o644)
            if fd >= 0 {
                close(fd)
                ownsLock = true
                return
            }
            if errno != EEXIST || Date() >= deadline {
                throw ColdbrewError.other("Timed out waiting for lock: \(path.path)")
            }
            Thread.sleep(forTimeInterval: 0.2)
        }
    }

    deinit {
        if ownsLock {
            try? FileManager.default.removeItem(at: path)
        }
    }
}
