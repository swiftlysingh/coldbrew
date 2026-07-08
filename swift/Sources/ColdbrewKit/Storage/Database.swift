import Foundation
import SQLite3

public let coldbrewSchemaVersion: Int32 = 3

public struct ApiCacheEntry: Equatable, Sendable {
    public var etag: String?
    public var lastModified: String?
    public var cachedAt: Int64
}

public struct BlobCacheEntry: Equatable, Sendable {
    public var sha256: String
    public var name: String?
    public var version: String?
    public var tag: String?
    public var sizeBytes: UInt64
    public var createdAt: Int64
}

public struct StoreEntryInfo: Equatable, Sendable {
    public var sha256: String
    public var sizeBytes: UInt64
    public var createdAt: Int64
}

public struct Database: Sendable {
    public let path: URL

    public init(paths: Paths) {
        self.path = paths.dbFile
    }

    public func connect() throws -> SQLiteConnection {
        try FileManager.default.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
        let connection = try SQLiteConnection(path: path)
        try configure(connection)
        try migrate(connection)
        return connection
    }

    public func getApiCache(_ connection: SQLiteConnection, url: String) throws -> ApiCacheEntry? {
        try connection.firstRow(
            "SELECT etag, last_modified, cached_at FROM api_cache WHERE url = ?1",
            [.text(url)]
        ).map {
            ApiCacheEntry(etag: $0.text(0), lastModified: $0.text(1), cachedAt: $0.int64(2))
        }
    }

    public func upsertApiCache(
        _ connection: SQLiteConnection,
        url: String,
        etag: String?,
        lastModified: String?
    ) throws {
        try connection.execute(
            """
            INSERT INTO api_cache (url, etag, last_modified, cached_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(url) DO UPDATE
            SET etag = excluded.etag,
                last_modified = excluded.last_modified,
                cached_at = excluded.cached_at
            """,
            [.text(url), .optionalText(etag), .optionalText(lastModified), .int64(nowTimestamp())]
        )
    }

    public func upsertBlobCache(
        _ connection: SQLiteConnection,
        sha256: String,
        name: String?,
        version: String?,
        tag: String?,
        sizeBytes: UInt64
    ) throws {
        try connection.execute(
            """
            INSERT INTO blob_cache (sha256, name, version, tag, size_bytes, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(sha256) DO UPDATE
            SET name = excluded.name,
                version = excluded.version,
                tag = excluded.tag,
                size_bytes = excluded.size_bytes,
                created_at = excluded.created_at
            """,
            [
                .text(sha256),
                .optionalText(name),
                .optionalText(version),
                .optionalText(tag),
                .uint64(sizeBytes),
                .int64(nowTimestamp()),
            ]
        )
    }

    public func deleteBlobCache(_ connection: SQLiteConnection, sha256: String) throws {
        try connection.execute("DELETE FROM blob_cache WHERE sha256 = ?1", [.text(sha256)])
    }

    public func listBlobCache(_ connection: SQLiteConnection) throws -> [BlobCacheEntry] {
        try connection.rows(
            """
            SELECT sha256, name, version, tag, size_bytes, created_at
            FROM blob_cache
            ORDER BY name, version
            """
        ).map {
            BlobCacheEntry(
                sha256: $0.text(0) ?? "",
                name: $0.text(1),
                version: $0.text(2),
                tag: $0.text(3),
                sizeBytes: UInt64($0.int64(4)),
                createdAt: $0.int64(5)
            )
        }
    }

    public func upsertStoreEntry(_ connection: SQLiteConnection, sha256: String, sizeBytes: UInt64) throws {
        try connection.execute(
            """
            INSERT INTO store_entries (sha256, size_bytes, created_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(sha256) DO UPDATE
            SET size_bytes = excluded.size_bytes
            """,
            [.text(sha256), .uint64(sizeBytes), .int64(nowTimestamp())]
        )
    }

    public func addStoreRef(_ connection: SQLiteConnection, sha256: String, package: String, version: String) throws {
        try connection.execute(
            """
            INSERT OR REPLACE INTO store_refs (sha256, package, version, installed_at)
            VALUES (?1, ?2, ?3, ?4)
            """,
            [.text(sha256), .text(package), .text(version), .int64(nowTimestamp())]
        )
    }

    public func removeStoreRef(_ connection: SQLiteConnection, sha256: String, package: String, version: String) throws {
        try connection.execute(
            "DELETE FROM store_refs WHERE sha256 = ?1 AND package = ?2 AND version = ?3",
            [.text(sha256), .text(package), .text(version)]
        )
    }

    public func listOrphanedStoreEntries(_ connection: SQLiteConnection) throws -> [StoreEntryInfo] {
        try connection.rows(
            """
            SELECT se.sha256, se.size_bytes, se.created_at
            FROM store_entries se
            LEFT JOIN store_refs sr ON se.sha256 = sr.sha256
            WHERE sr.sha256 IS NULL
            ORDER BY se.created_at
            """
        ).map {
            StoreEntryInfo(
                sha256: $0.text(0) ?? "",
                sizeBytes: UInt64($0.int64(1)),
                createdAt: $0.int64(2)
            )
        }
    }

    public func deleteStoreEntry(_ connection: SQLiteConnection, sha256: String) throws {
        try connection.execute("DELETE FROM store_entries WHERE sha256 = ?1", [.text(sha256)])
    }

    private func configure(_ connection: SQLiteConnection) throws {
        try connection.execute("PRAGMA journal_mode = WAL")
        try connection.execute("PRAGMA synchronous = NORMAL")
        try connection.execute("PRAGMA foreign_keys = ON")
        try connection.execute("PRAGMA busy_timeout = 5000")
    }

    private func migrate(_ connection: SQLiteConnection) throws {
        var version = try Int32(connection.int64("PRAGMA user_version") ?? 0)

        if version < 1 {
            try connection.execute(
                """
                CREATE TABLE IF NOT EXISTS api_cache (
                    url TEXT PRIMARY KEY,
                    etag TEXT,
                    last_modified TEXT,
                    cached_at INTEGER NOT NULL
                )
                """
            )
            try connection.execute("PRAGMA user_version = 1")
            version = 1
        }

        if version < 2 {
            try connection.execute(
                """
                CREATE TABLE IF NOT EXISTS blob_cache (
                    sha256 TEXT PRIMARY KEY,
                    name TEXT,
                    version TEXT,
                    tag TEXT,
                    size_bytes INTEGER NOT NULL,
                    created_at INTEGER NOT NULL
                )
                """
            )
            try connection.execute("PRAGMA user_version = 2")
            version = 2
        }

        if version < 3 {
            try connection.execute(
                """
                CREATE TABLE IF NOT EXISTS store_entries (
                    sha256 TEXT PRIMARY KEY,
                    size_bytes INTEGER NOT NULL,
                    created_at INTEGER NOT NULL
                )
                """
            )
            try connection.execute(
                """
                CREATE TABLE IF NOT EXISTS store_refs (
                    sha256 TEXT NOT NULL,
                    package TEXT NOT NULL,
                    version TEXT NOT NULL,
                    installed_at INTEGER NOT NULL,
                    PRIMARY KEY (sha256, package, version)
                )
                """
            )
            try connection.execute("CREATE INDEX IF NOT EXISTS store_refs_sha_idx ON store_refs(sha256)")
            try connection.execute("PRAGMA user_version = 3")
            version = 3
        }

        if version != coldbrewSchemaVersion {
            try connection.execute("PRAGMA user_version = \(coldbrewSchemaVersion)")
        }
    }
}

public final class SQLiteConnection {
    private let db: OpaquePointer

    fileprivate init(path: URL) throws {
        var handle: OpaquePointer?
        guard sqlite3_open(path.path, &handle) == SQLITE_OK, let handle else {
            let message = handle.map { String(cString: sqlite3_errmsg($0)) } ?? "unable to open database"
            if let handle {
                sqlite3_close(handle)
            }
            throw ColdbrewError.database(message)
        }
        self.db = handle
        sqlite3_busy_timeout(db, 5000)
    }

    deinit {
        sqlite3_close(db)
    }

    public func execute(_ sql: String, _ values: [SQLiteValue] = []) throws {
        if values.isEmpty {
            var error: UnsafeMutablePointer<CChar>?
            if sqlite3_exec(db, sql, nil, nil, &error) != SQLITE_OK {
                let message = error.map { String(cString: $0) } ?? lastError
                sqlite3_free(error)
                throw ColdbrewError.database(message)
            }
            return
        }

        let statement = try prepare(sql, values)
        defer { sqlite3_finalize(statement) }
        let result = sqlite3_step(statement)
        guard result == SQLITE_DONE else {
            throw ColdbrewError.database(lastError)
        }
    }

    func rows(_ sql: String, _ values: [SQLiteValue] = []) throws -> [SQLiteRow] {
        let statement = try prepare(sql, values)
        defer { sqlite3_finalize(statement) }

        var rows: [SQLiteRow] = []
        while true {
            let result = sqlite3_step(statement)
            if result == SQLITE_DONE {
                return rows
            }
            guard result == SQLITE_ROW else {
                throw ColdbrewError.database(lastError)
            }
            rows.append(SQLiteRow(statement: statement))
        }
    }

    func firstRow(_ sql: String, _ values: [SQLiteValue] = []) throws -> SQLiteRow? {
        try rows(sql, values).first
    }

    func int64(_ sql: String) throws -> Int64? {
        try firstRow(sql)?.int64(0)
    }

    func string(_ sql: String) throws -> String? {
        try firstRow(sql)?.text(0)
    }

    private func prepare(_ sql: String, _ values: [SQLiteValue]) throws -> OpaquePointer {
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK, let statement else {
            throw ColdbrewError.database(lastError)
        }

        do {
            try bind(values, to: statement)
            return statement
        } catch {
            sqlite3_finalize(statement)
            throw error
        }
    }

    private func bind(_ values: [SQLiteValue], to statement: OpaquePointer) throws {
        for (offset, value) in values.enumerated() {
            let index = Int32(offset + 1)
            let result: Int32
            switch value {
            case .null:
                result = sqlite3_bind_null(statement, index)
            case .text(let string):
                result = sqlite3_bind_text(statement, index, string, -1, sqliteTransient)
            case .int64(let int):
                result = sqlite3_bind_int64(statement, index, int)
            }
            guard result == SQLITE_OK else {
                throw ColdbrewError.database(lastError)
            }
        }
    }

    private var lastError: String {
        String(cString: sqlite3_errmsg(db))
    }
}

public enum SQLiteValue {
    case null
    case text(String)
    case int64(Int64)

    static func optionalText(_ value: String?) -> SQLiteValue {
        value.map(SQLiteValue.text) ?? .null
    }

    static func uint64(_ value: UInt64) -> SQLiteValue {
        .int64(Int64(value))
    }
}

struct SQLiteRow {
    private let values: [SQLiteColumn]

    init(statement: OpaquePointer) {
        values = (0..<sqlite3_column_count(statement)).map { index in
            if sqlite3_column_type(statement, index) == SQLITE_NULL {
                return .null
            }
            if let text = sqlite3_column_text(statement, index) {
                return .text(String(cString: text))
            }
            return .int64(sqlite3_column_int64(statement, index))
        }
    }

    func text(_ index: Int) -> String? {
        guard case .text(let text) = values[index] else { return nil }
        return text
    }

    func int64(_ index: Int) -> Int64 {
        switch values[index] {
        case .int64(let int):
            return int
        case .text(let text):
            return Int64(text) ?? 0
        case .null:
            return 0
        }
    }
}

private enum SQLiteColumn {
    case null
    case text(String)
    case int64(Int64)
}

private let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

private func nowTimestamp() -> Int64 {
    Int64(Date().timeIntervalSince1970)
}
