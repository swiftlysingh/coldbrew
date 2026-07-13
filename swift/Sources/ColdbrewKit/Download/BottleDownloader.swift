import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

public struct BottleDownloadRequest: Equatable, Sendable {
    public var url: URL
    public var sha256: String
    public var name: String?
    public var version: String?
    public var tag: String?

    public init(url: URL, sha256: String, name: String? = nil, version: String? = nil, tag: String? = nil) {
        self.url = url
        self.sha256 = sha256
        self.name = name
        self.version = version
        self.tag = tag
    }
}

public struct BottleDownloadResult: Equatable, Sendable {
    public var path: URL
    public var bytesDownloaded: UInt64
    public var downloaded: Bool
}

public final class BottleDownloader: Sendable {
    private static let cachePromotionLock = NSLock()
    private let cache: Cache
    private let session: URLSession
    private let tokenURL: URL

    public init(cache: Cache, session: URLSession = .shared, tokenURL: URL = URL(string: "https://ghcr.io/token")!) {
        self.cache = cache
        self.session = session
        self.tokenURL = tokenURL
    }

    public func downloadToCache(
        _ request: BottleDownloadRequest,
        progress: @Sendable (UInt64, UInt64) -> Void = { _, _ in }
    ) async throws -> BottleDownloadResult {
        if let cached = cache.cachedPath(sha256: request.sha256), try SHA256.verify(file: cached, expected: request.sha256) {
            let size = try cached.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
            try cache.recordBlobMetadata(
                sha256: request.sha256,
                name: request.name,
                version: request.version,
                tag: request.tag,
                sizeBytes: UInt64(size)
            )
            return BottleDownloadResult(path: cached, bytesDownloaded: 0, downloaded: false)
        }

        try cache.initialize()
        let tempURL = cache.blobTempPath(sha256: request.sha256).appendingPathExtension(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: tempURL) }

        let bytesDownloaded = try await download(request.url, to: tempURL, progress: progress)
        try SHA256.verifyBottle(file: tempURL, expected: request.sha256, package: request.name ?? "bottle")
        let cached = try promoteToCache(tempURL, sha256: request.sha256)
        try cache.recordBlobMetadata(
            sha256: request.sha256,
            name: request.name,
            version: request.version,
            tag: request.tag,
            sizeBytes: bytesDownloaded
        )
        return BottleDownloadResult(path: cached, bytesDownloaded: bytesDownloaded, downloaded: true)
    }

    public func fetchGhcrToken(repository: String) async throws -> String {
        var components = URLComponents(url: tokenURL, resolvingAgainstBaseURL: false)
        components?.queryItems = [
            URLQueryItem(name: "service", value: "ghcr.io"),
            URLQueryItem(name: "scope", value: "repository:\(repository):pull"),
        ]
        guard let url = components?.url else {
            throw ColdbrewError.ghcrAuthFailed("Invalid token URL")
        }

        let (data, response) = try await session.data(from: url)
        try validateHTTP(response, failure: ColdbrewError.ghcrAuthFailed("Token request failed"))
        let token = try JSONDecoder().decode(TokenResponse.self, from: data).token
        return token
    }

    public static func repository(fromBottleURL url: URL) throws -> String {
        let path = url.path
        guard let rangeStart = path.range(of: "/v2/") else {
            throw ColdbrewError.ghcrAuthFailed("Bottle URL missing /v2/ segment")
        }
        let afterV2 = path[rangeStart.upperBound...]
        guard let rangeEnd = afterV2.range(of: "/blobs/") else {
            throw ColdbrewError.ghcrAuthFailed("Bottle URL missing /blobs/ segment")
        }
        let repository = String(afterV2[..<rangeEnd.lowerBound])
        guard !repository.isEmpty else {
            throw ColdbrewError.ghcrAuthFailed("Bottle URL has empty repository")
        }
        return repository
    }

    private func download(
        _ url: URL,
        to destination: URL,
        progress: @Sendable (UInt64, UInt64) -> Void
    ) async throws -> UInt64 {
        if url.isFileURL {
            try FileManager.default.copyItem(at: url, to: destination)
            let size = UInt64(try destination.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0)
            progress(size, size)
            return size
        }

        var request = URLRequest(url: url)
        if url.host == "ghcr.io" {
            let repository = try Self.repository(fromBottleURL: url)
            let token = try await fetchGhcrToken(repository: repository)
            request.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        }

        let (downloadedURL, response) = try await session.download(for: request)
        try validateHTTP(response, failure: ColdbrewError.downloadFailed("Bottle download failed"))
        try FileManager.default.moveItem(at: downloadedURL, to: destination)
        let size = UInt64(try destination.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0)
        let total = response.expectedContentLength > 0 ? UInt64(response.expectedContentLength) : size
        progress(size, total)
        return size
    }

    private func promoteToCache(_ tempURL: URL, sha256: String) throws -> URL {
        Self.cachePromotionLock.lock()
        defer { Self.cachePromotionLock.unlock() }

        if let cached = cache.cachedPath(sha256: sha256), try SHA256.verify(file: cached, expected: sha256) {
            return cached
        }
        return try cache.moveToCache(from: tempURL, sha256: sha256)
    }

    private func validateHTTP(_ response: URLResponse, failure: ColdbrewError) throws {
        guard let http = response as? HTTPURLResponse else {
            return
        }
        guard (200..<300).contains(http.statusCode) else {
            switch failure {
            case .ghcrAuthFailed:
                throw ColdbrewError.ghcrAuthFailed("Token request failed: \(http.statusCode)")
            default:
                throw ColdbrewError.downloadFailed("Bottle download failed: \(http.statusCode)")
            }
        }
    }
}

private struct TokenResponse: Decodable {
    var token: String
}
