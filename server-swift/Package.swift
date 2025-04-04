// swift-tools-version: 6.0
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "azookey-server",
    products: [
        // Products define the executables and libraries a package produces, making them visible to other packages.
        .library(
            name: "azookey-server",
            type: .dynamic,
            targets: ["azookey-server"]
        ),
        .library(name: "ffi", targets: ["azookey-server"])
    ],
    dependencies: [
        // Dependencies declare other packages that this package depends on.
        // .package(url: /* package url */, from: "1.0.0"),
        .package(url: "https://github.com/azookey/AzooKeyKanaKanjiConverter", branch: "7d5dd99")
    ],
    targets: [
        // Targets are the basic building blocks of a package, defining a module or a test suite.
        // Targets can depend on other targets in this package and products from dependencies.
        .target(name: "ffi"),
        .target(
            name: "azookey-server",
            dependencies: [
                .product(name: "KanaKanjiConverterModule", package: "azookeykanakanjiconverter"),
                "ffi"
            ]
        ),
        .testTarget(
            name: "azookey-serverTests",
            dependencies: ["azookey-server"]
        ),
    ]
)
