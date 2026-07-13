// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "Coldbrew",
    platforms: [
        .macOS(.v13),
    ],
    products: [
        .library(name: "ColdbrewKit", targets: ["ColdbrewKit"]),
        .executable(name: "crew", targets: ["crew"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.5.0"),
    ],
    targets: [
        .target(
            name: "ColdbrewKit",
            dependencies: ["CSQLite"],
            linkerSettings: [
                .linkedLibrary("sqlite3", .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "crew",
            dependencies: [
                "ColdbrewKit",
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ]
        ),
        .testTarget(
            name: "ColdbrewKitTests",
            dependencies: ["ColdbrewKit"]
        ),
        .systemLibrary(
            name: "CSQLite",
            pkgConfig: "sqlite3",
            providers: [
                .apt(["libsqlite3-dev"]),
            ]
        ),
    ]
)
