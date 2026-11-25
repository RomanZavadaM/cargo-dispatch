// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "CargoDispatchDriverCore",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(name: "DriverCore", targets: ["DriverCore"]),
    ],
    targets: [
        .target(
            name: "DriverCore",
            path: "Core/Sources/DriverCore"
        ),
        .testTarget(
            name: "DriverCoreTests",
            dependencies: ["DriverCore"],
            path: "Core/Tests/DriverCoreTests"
        ),
    ]
)
