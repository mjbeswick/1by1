import Foundation

/// How ratings are shown and chosen. The stored value is the same 0 to 5 either way.
enum RatingStyle: String, Codable, CaseIterable, Labeled {
    case stars, thumbs
    var label: String {
        switch self {
        case .stars: return "Stars"
        case .thumbs: return "Thumbs"
        }
    }
}

/// Track ratings from 0 (none) to 5, keyed by file path. Kept in ratings.json beside settings.json.
/// Thumbs up stores 5 and thumbs down stores 1.
final class Ratings {
    static let shared = Ratings()
    private var values: [String: Int] = [:]
    private static let fileURL: URL = Settings.fileURL.deletingLastPathComponent().appendingPathComponent("ratings.json")

    init() {
        if let data = try? Data(contentsOf: Self.fileURL),
           let saved = try? JSONDecoder().decode([String: Int].self, from: data) {
            values = saved
        }
    }

    func value(_ url: URL) -> Int { values[url.path] ?? 0 }

    func set(_ url: URL, _ value: Int) {
        values[url.path] = value <= 0 ? nil : min(5, value)
        save()
    }

    /// Carry a rating over when the file moves.
    func move(_ from: URL, to: URL) {
        guard let v = values.removeValue(forKey: from.path) else { return }
        values[to.path] = v
        save()
    }

    private func save() {
        guard let data = try? JSONEncoder().encode(values) else { return }
        try? data.write(to: Self.fileURL, options: .atomic)
    }
}

/// Style the list uses. Set from settings in applySettings.
var ratingStyleSetting: RatingStyle = .stars

/// Text for the rating column.
func ratingText(_ url: URL) -> String {
    let v = Ratings.shared.value(url)
    if v == 0 { return "" }
    switch ratingStyleSetting {
    case .stars: return String(repeating: "\u{2605}", count: v) + String(repeating: "\u{2606}", count: 5 - v)
    case .thumbs: return v >= 4 ? "\u{1F44D}" : (v <= 2 ? "\u{1F44E}" : "\u{2013}")
    }
}
