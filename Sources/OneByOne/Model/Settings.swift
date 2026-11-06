import Foundation

/// Persisted state. JSON in Application Support. Only the fields the native UI uses so far.
struct Settings: Codable {
    var roots: [String] = []
    var lastFolder: String?
    /// Per-folder resume: folder path to (file path, seconds).
    var folderResume: [String: Resume] = [:]
    var volume: Float = 1.0
    var muted = false
    var repeatMode: Repeat = .off
    var shuffle = false
    var treeVisible = true

    struct Resume: Codable {
        var path: String
        var seconds: Double
    }

    static let fileURL: URL = {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("1by1", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        return base.appendingPathComponent("settings.json")
    }()

    static func load() -> Settings {
        guard let data = try? Data(contentsOf: fileURL),
              let s = try? JSONDecoder().decode(Settings.self, from: data) else {
            return Settings()
        }
        return s
    }

    func save() {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? enc.encode(self) else { return }
        try? data.write(to: Settings.fileURL, options: .atomic)
    }
}
