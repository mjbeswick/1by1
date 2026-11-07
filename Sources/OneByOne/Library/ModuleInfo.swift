import COpenMPT
import Foundation

/// Runs `body` with a libopenmpt handle for the file, or returns nil when it cannot open.
private func withModule<T>(_ url: URL, _ body: (OpaquePointer, (String) -> String?) -> T) -> T? {
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
    return body(mod, meta)
}

/// Title, tracker, length and message of a tracker module. Read-only, as in the Rust build.
func moduleInfo(_ url: URL) -> AudioInfo? {
    withModule(url) { mod, meta in
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
}

/// Full module view: tracker type, structure counts and the song message. Label and value rows.
func moduleDetails(_ url: URL) -> (rows: [(String, String)], message: String) {
    withModule(url) { mod, meta -> (rows: [(String, String)], message: String) in
        let seconds = openmpt_module_get_duration_seconds(mod)
        var rows: [(String, String)] = [
            ("Type", meta("type_long") ?? "Tracker module"),
            ("Tracker", meta("tracker") ?? "--"),
            ("Title", meta("title") ?? "--"),
            ("Artist", meta("artist") ?? "--"),
            ("Date", meta("date") ?? "--"),
            ("Length", seconds > 0 ? clock(seconds) : "--:--"),
            ("Channels", "\(openmpt_module_get_num_channels(mod))"),
            ("Orders", "\(openmpt_module_get_num_orders(mod))"),
            ("Patterns", "\(openmpt_module_get_num_patterns(mod))"),
            ("Instruments", "\(openmpt_module_get_num_instruments(mod))"),
            ("Samples", "\(openmpt_module_get_num_samples(mod))"),
            ("Subsongs", "\(openmpt_module_get_num_subsongs(mod))"),
        ]
        if meta("date") == nil { rows.removeAll { $0.0 == "Date" } }
        let message = (meta("message") ?? "").replacingOccurrences(of: "\r", with: "")
        return (rows, message)
    } ?? (rows: [], message: "")
}
