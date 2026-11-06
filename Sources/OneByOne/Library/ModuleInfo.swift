import COpenMPT
import Foundation

/// Title, tracker, length and message of a tracker module. Read-only, as in the Rust build.
func moduleInfo(_ url: URL) -> AudioInfo? {
    guard let data = try? Data(contentsOf: url), !data.isEmpty else { return nil }
    let made: OpaquePointer? = data.withUnsafeBytes { raw in
        openmpt_module_create_from_memory2(raw.baseAddress, raw.count, nil, nil, nil, nil, nil, nil, nil)
    }
    guard let mod = made else { return nil }
    defer { openmpt_module_destroy(mod) }

    func meta(_ key: String) -> String? {
        guard let p = openmpt_module_get_metadata(mod, key) else { return nil }
        defer { openmpt_free_string(p) }
        let s = String(cString: p)
        return s.isEmpty ? nil : s
    }

    let ext = url.pathExtension.uppercased()
    let kind = meta("type_long")
    let tracker = meta("tracker")
    let format: String
    switch (kind, tracker) {
    case let (k?, t?) where k != t: format = "\(k), saved by \(t) (\(ext))"
    case let (k?, _): format = "\(k) (\(ext))"
    case let (nil, t?): format = "\(t) (\(ext))"
    default: format = "Tracker module (\(ext))"
    }

    let seconds = openmpt_module_get_duration_seconds(mod)
    var info = AudioInfo()
    info.duration = seconds > 0 ? seconds : nil
    info.sampleRate = 48_000
    info.channels = 2
    info.decoder = "Tracker"
    info.format = format
    info.tags.title = meta("title")
    info.tags.artist = meta("artist")
    info.tags.year = meta("date").map { String($0.prefix(4)) }
    info.tags.comment = meta("message")?.replacingOccurrences(of: "\r", with: "")
    return info
}
