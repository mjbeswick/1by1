import Foundation

/// Imports the Rust build's `settings.toml` (from `~/.config/1by1/`) into the native settings.
/// Runs once, when no native settings file exists yet. Bookmarks, resume points and options carry over.
enum RustSettingsImport {
    static var candidatePaths: [URL] {
        let home = FileManager.default.homeDirectoryForCurrentUser
        return [home.appendingPathComponent(".config/1by1/settings.toml")]
    }

    /// The first settings file found, if any.
    static func source() -> URL? {
        candidatePaths.first { FileManager.default.fileExists(atPath: $0.path) }
    }

    /// Settings built from the Rust file. Nil when there is no file or it cannot be read.
    static func importSettings(from url: URL) -> Settings? {
        guard let text = try? String(contentsOf: url, encoding: .utf8),
              let table = MiniTOML.parse(text) else { return nil }
        // Start from the native defaults and overwrite what the Rust file has.
        guard let enc = try? JSONEncoder().encode(Settings()),
              var json = (try? JSONSerialization.jsonObject(with: enc)) as? [String: Any] else { return nil }

        for (rustKey, value) in table {
            switch rustKey {
            case "folder_state":
                json["folderResume"] = resumeMap(value)
            case "bookmarks":
                json["bookmarks"] = bookmarks(value)
            case "last_dir":
                json["lastFolder"] = value
            case "last_file":
                json["lastFile"] = value
            case "last_pos":
                json["lastPos"] = value
            case "repeat":
                json["repeatMode"] = enumValue(value, ["off", "all", "one"])
            case "cover_mode":
                json["coverMode"] = enumValue(value, ["artwork", "off"], aliases: ["milkdrop": "artwork", "spectrum": "artwork"])
            case "playlist_pos":
                json["playlistPos"] = enumValue(value, ["top", "bottom", "right"])
            case "sort":
                json["sort"] = enumValue(value, ["name", "length", "size", "type", "modified", "created"])
            case "big_layout":
                json["bigLayout"] = enumValue(value, BigLayout.allCases.map(\.rawValue))
            case "title_mode":
                json["titleMode"] = enumValue(value, TitleMode.allCases.map(\.rawValue))
            case "info_mode":
                json["infoMode"] = enumValue(value, InfoMode.allCases.map(\.rawValue))
            case "external":
                json["external"] = value
            case "win_size":
                if let pair = value as? [Any], pair.count == 2 {
                    json["winWidth"] = pair[0]
                    json["winHeight"] = pair[1]
                }
            default:
                let key = camel(rustKey)
                if json[key] != nil { json[key] = value }
            }
        }
        guard let data = try? JSONSerialization.data(withJSONObject: json) else { return nil }
        return try? JSONDecoder().decode(Settings.self, from: data)
    }

    /// Run the import when there are no native settings yet.
    static func importIfFirstRun() -> Settings? {
        guard !FileManager.default.fileExists(atPath: Settings.fileURL.path),
              let src = source(), let s = importSettings(from: src) else { return nil }
        s.save()
        return s
    }

    // MARK: - Mapping helpers

    private static func camel(_ snake: String) -> String {
        let parts = snake.split(separator: "_")
        guard let first = parts.first else { return snake }
        return String(first) + parts.dropFirst().map { $0.prefix(1).uppercased() + $0.dropFirst() }.joined()
    }

    /// Rust folder_state is [folder, file, seconds]. Native resume is folder -> {path, seconds}.
    private static func resumeMap(_ v: Any) -> [String: Any] {
        var out: [String: Any] = [:]
        for case let row as [Any] in (v as? [Any] ?? []) where row.count == 3 {
            if let folder = row[0] as? String, let file = row[1] as? String {
                out[folder] = ["path": file, "seconds": row[2]]
            }
        }
        return out
    }

    private static func bookmarks(_ v: Any) -> [[String: Any]] {
        (v as? [Any] ?? []).compactMap { item in
            guard let t = item as? [String: Any], let path = t["path"] as? String else { return nil }
            var isDir: ObjCBool = false
            let folder = FileManager.default.fileExists(atPath: path, isDirectory: &isDir) && isDir.boolValue
            return ["title": t["title"] as? String ?? path, "path": path, "isFolder": folder]
        }
    }

    /// Rust enums serialise lowercase. Match case-insensitively onto the native raw values.
    private static func enumValue(_ v: Any, _ allowed: [String], aliases: [String: String] = [:]) -> Any {
        guard let s = (v as? String)?.lowercased() else { return v }
        if let a = aliases[s] { return a }
        return allowed.first { $0.lowercased() == s } ?? allowed[0]
    }
}

/// Reads the subset of TOML the Rust build writes: `key = value` lines, `[[table]]` arrays,
/// strings, numbers, booleans, and arrays (possibly spanning lines). Other syntax is skipped.
enum MiniTOML {
    static func parse(_ text: String) -> [String: Any]? {
        var root: [String: Any] = [:]
        var tableKey: String?
        var tables: [[String: Any]] = []
        var pending = ""
        var pendingKey: String?

        func store(_ key: String, _ value: Any) {
            if let t = tableKey {
                if tables.isEmpty { return }
                tables[tables.count - 1][key] = value
                _ = t
            } else {
                root[key] = value
            }
        }

        for rawLine in text.components(separatedBy: .newlines) {
            let line = stripComment(rawLine).trimmingCharacters(in: .whitespaces)
            if pendingKey != nil {
                pending += " " + line
                if balanced(pending) {
                    if let key = pendingKey, let v = Value(pending).parse() { store(key, v) }
                    pendingKey = nil
                    pending = ""
                }
                continue
            }
            if line.isEmpty { continue }
            if line.hasPrefix("[[") && line.hasSuffix("]]") {
                // Flush the previous array-of-tables entry.
                if let k = tableKey, !tables.isEmpty { root[k] = tables }
                tableKey = String(line.dropFirst(2).dropLast(2)).trimmingCharacters(in: .whitespaces)
                tables.append([:])
                continue
            }
            if line.hasPrefix("[") { continue }
            guard let eq = line.firstIndex(of: "=") else { continue }
            let key = String(line[..<eq]).trimmingCharacters(in: .whitespaces)
            let raw = String(line[line.index(after: eq)...]).trimmingCharacters(in: .whitespaces)
            if !balanced(raw) {
                pendingKey = key
                pending = raw
                continue
            }
            if let v = Value(raw).parse() { store(key, v) }
        }
        if let k = tableKey, !tables.isEmpty { root[k] = tables }
        return root
    }

    private static func stripComment(_ line: String) -> String {
        var inString = false
        for (i, c) in line.enumerated() {
            if c == "\"" { inString.toggle() }
            if c == "#" && !inString { return String(line.prefix(i)) }
        }
        return line
    }

    /// True when every `[` and string quote is closed.
    private static func balanced(_ s: String) -> Bool {
        var depth = 0
        var inString = false
        for c in s {
            if c == "\"" { inString.toggle() }
            if !inString {
                if c == "[" { depth += 1 }
                if c == "]" { depth -= 1 }
            }
        }
        return depth <= 0 && !inString
    }

    private struct Value {
        let s: String
        init(_ s: String) { self.s = s }

        func parse() -> Any? {
            var p = Parser(chars: Array(s.trimmingCharacters(in: .whitespaces)))
            return p.value()
        }
    }

    private struct Parser {
        let chars: [Character]
        var i = 0

        init(chars: [Character]) { self.chars = chars }

        mutating func skipSpace() {
            while i < chars.count, chars[i] == " " || chars[i] == "\t" || chars[i] == "," || chars[i] == "\n" { i += 1 }
        }

        mutating func value() -> Any? {
            skipSpace()
            guard i < chars.count else { return nil }
            switch chars[i] {
            case "\"": return string()
            case "[": return array()
            case "{": return nil
            default:
                var word = ""
                while i < chars.count, chars[i] != ",", chars[i] != "]", chars[i] != " " { word.append(chars[i]); i += 1 }
                if word == "true" { return true }
                if word == "false" { return false }
                if let n = Int(word) { return n }
                if let d = Double(word) { return d }
                return nil
            }
        }

        mutating func string() -> String {
            i += 1
            var out = ""
            while i < chars.count, chars[i] != "\"" {
                if chars[i] == "\\", i + 1 < chars.count {
                    i += 1
                    switch chars[i] {
                    case "n": out.append("\n")
                    case "t": out.append("\t")
                    default: out.append(chars[i])
                    }
                } else {
                    out.append(chars[i])
                }
                i += 1
            }
            i += 1
            return out
        }

        mutating func array() -> [Any] {
            i += 1
            var items: [Any] = []
            while true {
                skipSpace()
                guard i < chars.count else { break }
                if chars[i] == "]" { i += 1; break }
                guard let v = value() else { i += 1; continue }
                items.append(v)
            }
            return items
        }
    }
}
