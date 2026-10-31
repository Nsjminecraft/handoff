// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "handoff-ios",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(
            name: "HandoffiOS",
            targets: ["HandoffiOS"]
        ),
    ],
    targets: [
        .target(
            name: "HandoffiOS",
            path: "Sources"
        ),
    ]
)
