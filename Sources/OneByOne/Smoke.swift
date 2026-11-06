import Foundation

/// Headless check: feeds the given files gaplessly and prints what is heard.
/// Exits 0 when every file was heard, 1 on any open error or timeout.
func runAudioSmoke(paths: [String]) -> Never {
    let specs = paths.map { Spec.whole(URL(fileURLWithPath: $0)) }
    guard !specs.isEmpty else {
        print("usage: OneByOne --smoke <file>...")
        exit(1)
    }
    let engine = Engine()
    var cursor = 1  // specs[0] is started directly below
    var heard = 0
    engine.nextProvider = {
        guard cursor < specs.count else { return nil }
        defer { cursor += 1 }
        return specs[cursor]
    }
    engine.onTrackChanged = { spec in
        heard += 1
        print("heard \(heard)/\(specs.count) \(spec.url.lastPathComponent) pos=\(String(format: "%.2f", engine.position))")
    }
    engine.onError = { spec, msg in
        print("error \(spec.url.lastPathComponent): \(msg)")
        exit(1)
    }
    engine.onFinished = {
        print(heard == specs.count ? "finished ok" : "finished early: \(heard)/\(specs.count)")
        exit(heard == specs.count ? 0 : 1)
    }
    engine.play(specs[0])
    DispatchQueue.main.asyncAfter(deadline: .now() + 60) {
        print("timeout")
        exit(1)
    }
    RunLoop.main.run()
    exit(1)
}

/// Prints what the library sees in a folder. Used to check tags and durations.
func runLibraryScan(dir: String) -> Never {
    let tracks = folderTracks(URL(fileURLWithPath: dir))
    for t in tracks {
        t.info = scanInfo(t.url)
        let d = t.duration.map { clock($0) } ?? "--:--"
        print("\(d)  \(t.typeText)  \(t.display)  |  \(t.infoText)  |  \(t.info?.format ?? t.info?.decoder ?? "")")
    }
    print("\(tracks.count) tracks, \(subfolders(URL(fileURLWithPath: dir)).count) subfolders")
    exit(0)
}
