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
    targets: [
        .target(
            name: "ColdbrewKit",
            linkerSettings: [
                .linkedLibrary("sqlite3"),
            ]
        ),
        .executableTarget(
            name: "crew",
            dependencies: ["ColdbrewKit"]
        ),
        .testTarget(
            name: "ColdbrewKitTests",
            dependencies: ["ColdbrewKit"]
        ),
    ]
)
