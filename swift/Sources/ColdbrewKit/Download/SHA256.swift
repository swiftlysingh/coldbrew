import Foundation

public enum SHA256 {
    public static func hash(_ data: Data) -> String {
        digest(Array(data)).map { String(format: "%02x", $0) }.joined()
    }

    public static func hash(file url: URL) throws -> String {
        guard let stream = InputStream(url: url) else {
            throw ColdbrewError.pathNotFound(url.path)
        }
        stream.open()
        defer { stream.close() }

        var hasher = IncrementalSHA256()
        var buffer = [UInt8](repeating: 0, count: 8192)
        while stream.hasBytesAvailable {
            let count = stream.read(&buffer, maxLength: buffer.count)
            if count < 0 {
                throw stream.streamError.map { ColdbrewError.io($0.localizedDescription) } ?? ColdbrewError.io("Failed to read \(url.path)")
            }
            if count == 0 {
                break
            }
            hasher.update(buffer[..<count])
        }
        return hasher.finalize().map { String(format: "%02x", $0) }.joined()
    }

    public static func verify(file url: URL, expected: String) throws -> Bool {
        try hash(file: url) == expected.lowercased()
    }

    public static func verifyBottle(file url: URL, expected: String, package: String) throws {
        let actual = try hash(file: url)
        guard actual == expected.lowercased() else {
            throw ColdbrewError.checksumMismatch(package: package, expected: expected, actual: actual)
        }
    }

    private static func digest(_ bytes: [UInt8]) -> [UInt8] {
        var hasher = IncrementalSHA256()
        hasher.update(bytes[...])
        return hasher.finalize()
    }
}

private struct IncrementalSHA256 {
    private var h: [UInt32] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ]
    private var buffer: [UInt8] = []
    private var byteCount: UInt64 = 0

    mutating func update<S: Collection>(_ bytes: S) where S.Element == UInt8 {
        byteCount += UInt64(bytes.count)
        buffer.append(contentsOf: bytes)
        while buffer.count >= 64 {
            process(Array(buffer.prefix(64)))
            buffer.removeFirst(64)
        }
    }

    mutating func finalize() -> [UInt8] {
        let bitCount = byteCount * 8
        buffer.append(0x80)
        while buffer.count % 64 != 56 {
            buffer.append(0)
        }
        for shift in stride(from: 56, through: 0, by: -8) {
            buffer.append(UInt8((bitCount >> UInt64(shift)) & 0xff))
        }
        while !buffer.isEmpty {
            process(Array(buffer.prefix(64)))
            buffer.removeFirst(64)
        }

        return h.flatMap { word in
            [
                UInt8((word >> 24) & 0xff),
                UInt8((word >> 16) & 0xff),
                UInt8((word >> 8) & 0xff),
                UInt8(word & 0xff),
            ]
        }
    }

    private mutating func process(_ chunk: [UInt8]) {
        var w = [UInt32](repeating: 0, count: 64)
        for index in 0..<16 {
            let offset = index * 4
            w[index] =
                UInt32(chunk[offset]) << 24 |
                UInt32(chunk[offset + 1]) << 16 |
                UInt32(chunk[offset + 2]) << 8 |
                UInt32(chunk[offset + 3])
        }
        for index in 16..<64 {
            let s0 = rotateRight(w[index - 15], by: 7) ^ rotateRight(w[index - 15], by: 18) ^ (w[index - 15] >> 3)
            let s1 = rotateRight(w[index - 2], by: 17) ^ rotateRight(w[index - 2], by: 19) ^ (w[index - 2] >> 10)
            w[index] = w[index - 16] &+ s0 &+ w[index - 7] &+ s1
        }

        var a = h[0]
        var b = h[1]
        var c = h[2]
        var d = h[3]
        var e = h[4]
        var f = h[5]
        var g = h[6]
        var hh = h[7]

        for index in 0..<64 {
            let s1 = rotateRight(e, by: 6) ^ rotateRight(e, by: 11) ^ rotateRight(e, by: 25)
            let ch = (e & f) ^ (~e & g)
            let temp1 = hh &+ s1 &+ ch &+ k[index] &+ w[index]
            let s0 = rotateRight(a, by: 2) ^ rotateRight(a, by: 13) ^ rotateRight(a, by: 22)
            let maj = (a & b) ^ (a & c) ^ (b & c)
            let temp2 = s0 &+ maj

            hh = g
            g = f
            f = e
            e = d &+ temp1
            d = c
            c = b
            b = a
            a = temp1 &+ temp2
        }

        h[0] = h[0] &+ a
        h[1] = h[1] &+ b
        h[2] = h[2] &+ c
        h[3] = h[3] &+ d
        h[4] = h[4] &+ e
        h[5] = h[5] &+ f
        h[6] = h[6] &+ g
        h[7] = h[7] &+ hh
    }
}

private func rotateRight(_ value: UInt32, by bits: UInt32) -> UInt32 {
    (value >> bits) | (value << (32 - bits))
}

private let k: [UInt32] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
]
