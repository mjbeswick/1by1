import AppKit

let args = Array(CommandLine.arguments.dropFirst())

if args.first == "--selftest" {
    exit(runLogicSelfTest() ? 0 : 1)
}
if args.first == "--scan", args.count > 1 {
    runLibraryScan(dir: args[1])
}
if args.first == "--import-check", let src = RustSettingsImport.source() {
    if let s = RustSettingsImport.importSettings(from: src) {
        print("roots=\(s.roots) lastFolder=\(s.lastFolder ?? "-") volume=\(s.volume) repeat=\(s.repeatMode) rewind=\(s.rewindSecs)")
        print("resume folders=\(s.folderResume.count) bookmarks=\(s.bookmarks.count) gapless=\(s.gapless) remember=\(s.rememberPositions)")
    } else {
        print("import failed")
    }
    exit(0)
}

if args.first == "--smoke" {
    runAudioSmoke(paths: Array(args.dropFirst()))
}

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
