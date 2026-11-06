// swift-tools-version:6.0
import PackageDescription

let package = Package(
    name: "OneByOne",
    platforms: [.macOS(.v14)],
    targets: [
        .systemLibrary(name: "COpenMPT", pkgConfig: "libopenmpt"),
        .systemLibrary(name: "CVorbis", pkgConfig: "vorbisfile"),
        .systemLibrary(name: "CTagLib", pkgConfig: "taglib_c"),
        .systemLibrary(name: "CFFmpeg", pkgConfig: "libavformat"),
        .executableTarget(
            name: "OneByOne",
            dependencies: ["COpenMPT", "CVorbis", "CTagLib", "CFFmpeg"],
            path: "Sources/OneByOne",
            swiftSettings: [.swiftLanguageMode(.v5)],
            linkerSettings: [.linkedLibrary("ogg"), .linkedLibrary("avcodec"), .linkedLibrary("avutil"), .linkedLibrary("swresample")]
        ),
    ]
)
