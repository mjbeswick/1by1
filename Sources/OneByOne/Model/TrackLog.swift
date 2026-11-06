import Foundation

/// Appends one line per heard track to `1by1.log` beside the settings file.
/// The file is cleared once it passes the size limit in settings.
enum TrackLog {
    static var url: URL {
        Settings.fileURL.deletingLastPathComponent().appendingPathComponent("1by1.log")
    }

    static func append(_ track: Track, maxKB: Int) {
        let line = "\(dateTimeText(Date()))  \(track.artistTitle)\t\(track.url.path)\n"
        if let data = line.data(using: .utf8) {
            if let h = try? FileHandle(forWritingTo: url) {
                h.seekToEndOfFile()
                h.write(data)
                try? h.close()
            } else {
                try? data.write(to: url)
            }
        }
        if maxKB > 0,
           let size = (try? FileManager.default.attributesOfItem(atPath: url.path))?[.size] as? NSNumber,
           size.intValue > maxKB * 1024 {
            try? "".write(to: url, atomically: true, encoding: .utf8)
        }
    }

    static func text() -> String {
        (try? String(contentsOf: url, encoding: .utf8)) ?? ""
    }
}
