import AVFoundation
import CTagLib
import Foundation

/// Extensions the app treats as audio. Modules go to libopenmpt, the rest to `openSource`.
let audioExtensions: Set<String> = [
    "mp1", "mp2", "mp3", "mpa", "mpga", "aac", "adts", "m4a", "m4b", "m4p", "m4r", "mp4",
    "flac", "ogg", "oga", "wav", "wave", "bwf", "aif", "aiff", "aifc", "caf",
    "mkv", "mka", "webm", "opus",
]


/// Settings read by list text. Set from the settings window.
var displayUsesTags = true
var infoModeSetting: InfoMode = .artistTitle

struct Tags: Equatable {
    var title: String?
    var artist: String?
    var album: String?
    var year: String?
    var track: String?
    var comment: String?

    var isEmpty: Bool { title == nil && artist == nil && album == nil }

    /// `Artist - Title`, else title, else artist.
    var artistTitle: String? {
        switch (artist, title) {
        case let (a?, t?) where !a.isEmpty && !t.isEmpty && a != t: return "\(a) - \(t)"
        case let (_, t?) where !t.isEmpty: return t
        case let (a?, _) where !a.isEmpty: return a
        default: return nil
        }
    }
}

struct AudioInfo {
    var duration: Double?
    var bitrate: Int?
    var sampleRate: Double?
    var channels: Int?
    var tags = Tags()
    var decoder = "Audio"
    /// Tracker or container description, e.g. "Impulse Tracker (IT)".
    var format: String?

    /// `160 kbps, 44.1 kHz, Stereo` style summary.
    var summary: String {
        var parts: [String] = []
        if let b = bitrate { parts.append("\(b) kbps") }
        if let s = sampleRate { parts.append(String(format: "%.1f kHz", s / 1000)) }
        if let c = channels { parts.append(channelsText(c)) }
        return parts.joined(separator: ", ")
    }
}

func channelsText(_ c: Int) -> String {
    switch c {
    case 1: return "Mono"
    case 2: return "Stereo"
    default: return "Multichannel"
    }
}

/// One row in the track list. A cue sheet entry shares `url` with its siblings.
final class Track {
    let url: URL
    let name: String
    let ext: String
    let size: UInt64
    let modified: Date?
    let created: Date?
    var info: AudioInfo?
    /// Cue virtual track: start and optional end inside `url`, in seconds.
    var cue: (start: Double, end: Double?)?

    init(url: URL) {
        self.url = url
        name = url.lastPathComponent
        ext = url.pathExtension.lowercased()
        let attrs = try? FileManager.default.attributesOfItem(atPath: url.path)
        size = (attrs?[.size] as? NSNumber)?.uint64Value ?? 0
        modified = attrs?[.modificationDate] as? Date
        created = attrs?[.creationDate] as? Date
    }

    var isCue: Bool { cue != nil }

    /// A subfolder row, shown when Folders in List is on. Activating it opens the folder.
    var isFolder = false

    /// Tag title when present, else file name.
    /// Tag title when tag titles are on and present, else file name.
    var display: String {
        if displayUsesTags, let t = info?.tags.title, !t.isEmpty { return t }
        return name
    }

    var artistTitle: String {
        info?.tags.artistTitle ?? name
    }

    var duration: Double? {
        if let (start, end) = cue {
            if let e = end { return max(0, e - start) }
            return info?.duration.map { max(0, $0 - start) }
        }
        return info?.duration
    }

    var typeText: String { isFolder ? "DIR" : isCue ? "CUE" : ext.uppercased() }

    /// Info column text, as chosen in settings.
    var infoText: String {
        guard let i = info else { return "" }
        let bitrate = i.bitrate.map { "\($0) kbps" } ?? ""
        switch infoModeSetting {
        case .bitrate: return i.summary
        case .artistTitle:
            if !i.tags.isEmpty, let at = i.tags.artistTitle { return at }
            return i.summary
        case .trackNo: return i.tags.track ?? ""
        case .length: return duration.map { clock($0) } ?? ""
        case .bitrateFmt: return [bitrate, i.format ?? i.decoder].filter { !$0.isEmpty }.joined(separator: ", ")
        case .bitrateTag: return [bitrate, i.tags.title ?? ""].filter { !$0.isEmpty }.joined(separator: ", ")
        case .yearAlbum: return [i.tags.year ?? "", i.tags.album ?? ""].filter { !$0.isEmpty }.joined(separator: " ")
        case .notes: return i.tags.comment ?? ""
        }
    }

    /// What the engine plays for this row.
    var spec: Spec {
        guard let (start, end) = cue else { return .whole(url) }
        return Spec(url: url, start: start, limit: end.map { $0 - start })
    }
}

// MARK: - Folders

/// Visible entries of `dir` that pass `keep`, in natural order.
private func entries(_ dir: URL, files: Bool, keep: (URL) -> Bool) -> [URL] {
    let items = (try? FileManager.default.contentsOfDirectory(
        at: dir,
        includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey],
        options: [.skipsHiddenFiles])) ?? []
    return items
        .filter { url in
            let v = try? url.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])
            if v?.isSymbolicLink == true { return false }
            return (v?.isDirectory ?? false) == !files
        }
        .filter(keep)
        .sorted { naturalCompare($0.path, $1.path) == .orderedAscending }
}

func isModule(_ url: URL) -> Bool { moduleExtensions.contains(url.pathExtension.lowercased()) }
func isAudio(_ url: URL) -> Bool {
    let e = url.pathExtension.lowercased()
    return audioExtensions.contains(e) || moduleExtensions.contains(e)
}
func isPlaylist(_ url: URL) -> Bool { ["m3u", "m3u8"].contains(url.pathExtension.lowercased()) }
func isCueSheet(_ url: URL) -> Bool { url.pathExtension.lowercased() == "cue" }

/// Subfolders directly in `dir`, natural order.
func subfolders(_ dir: URL) -> [URL] {
    entries(dir, files: false) { _ in true }
}

/// Rows for a folder. Subfolders come first when `withFolders` is set.
func folderRows(_ dir: URL, withFolders: Bool) -> [Track] {
    var out: [Track] = []
    if withFolders {
        for sub in subfolders(dir) {
            let t = Track(url: sub)
            t.isFolder = true
            out.append(t)
        }
    }
    return out + folderTracks(dir)
}

/// Audio files, cue sheets expanded into virtual tracks, and playlist entries.
func folderTracks(_ dir: URL) -> [Track] {
    var out: [Track] = []
    for url in entries(dir, files: true, keep: { isAudio($0) || isCueSheet($0) || isPlaylist($0) }) {
        if isCueSheet(url) {
            out += cueTracks(url)
        } else if isPlaylist(url) {
            out += readPlaylist(url).map(Track.init(url:))
        } else {
            out.append(Track(url: url))
        }
    }
    return out
}

// MARK: - Playlists and cue sheets

/// Entries of an m3u/m3u8 list. Relative paths resolve against the list's folder.
func readPlaylist(_ url: URL) -> [URL] {
    guard let text = try? String(contentsOf: url, encoding: .utf8) ??
        String(contentsOf: url, encoding: .isoLatin1) else { return [] }
    let base = url.deletingLastPathComponent()
    return text.split(whereSeparator: \.isNewline)
        .map { $0.trimmingCharacters(in: .whitespaces) }
        .filter { !$0.isEmpty && !$0.hasPrefix("#") }
        .map { line -> URL in
            line.hasPrefix("/") ? URL(fileURLWithPath: line) : base.appendingPathComponent(line)
        }
        .filter { FileManager.default.fileExists(atPath: $0.path) }
}

struct CueEntry {
    var title: String?
    var performer: String?
    var start: Double
}

/// Parses a cue sheet. Returns the audio file it points at and its tracks.
func readCue(_ url: URL) -> (audio: URL?, entries: [CueEntry]) {
    guard let text = try? String(contentsOf: url, encoding: .utf8) ??
        String(contentsOf: url, encoding: .isoLatin1) else { return (nil, []) }
    let base = url.deletingLastPathComponent()
    var audio: URL?
    var out: [CueEntry] = []
    var title: String?, performer: String?
    for raw in text.split(whereSeparator: \.isNewline) {
        let line = raw.trimmingCharacters(in: .whitespaces)
        let parts = line.split(separator: " ", maxSplits: 1, omittingEmptySubsequences: true)
        guard let keyPart = parts.first else { continue }
        let key = keyPart.uppercased()
        let rest = parts.count > 1
            ? String(parts[1]).trimmingCharacters(in: .whitespaces).trimmingCharacters(in: CharacterSet(charactersIn: "\""))
            : ""
        switch key {
        case "FILE":
            var name = rest
            for suffix in [" WAVE", " MP3", " AIFF", " BINARY"] where name.hasSuffix(suffix) {
                name = String(name.dropLast(suffix.count))
            }
            let p = name.trimmingCharacters(in: CharacterSet(charactersIn: "\""))
            audio = p.hasPrefix("/") ? URL(fileURLWithPath: p) : base.appendingPathComponent(p)
        case "TRACK":
            out.append(CueEntry(title: title, performer: performer, start: 0))
            title = nil
            performer = nil
        case "TITLE": title = rest
        case "PERFORMER": performer = rest
        case "INDEX":
            let f = rest.split(separator: " ")
            if f.count >= 2, f[0] == "01", !out.isEmpty, let t = parseCueTime(String(f[1])) {
                out[out.count - 1].start = t
            }
        default: break
        }
    }
    return (audio, out)
}

/// `mm:ss:ff` at 75 frames per second.
func parseCueTime(_ s: String) -> Double? {
    let p = s.split(separator: ":").compactMap { Int($0) }
    guard p.count >= 2 else { return nil }
    let frames = p.count > 2 ? p[2] : 0
    return Double(p[0] * 60 + p[1]) + Double(frames) / 75
}

/// Expands a cue sheet into one row per TRACK. Needs the audio length for the last entry.
func cueTracks(_ url: URL) -> [Track] {
    let (audio, entries) = readCue(url)
    guard let audio else { return [] }
    let total = scanInfo(audio)?.duration
    var out: [Track] = []
    for (i, e) in entries.enumerated() {
        let end = i + 1 < entries.count ? entries[i + 1].start : total
        let t = Track(url: audio)
        t.cue = (e.start, end)
        var info = AudioInfo()
        info.duration = end.map { max(0, $0 - e.start) }
        info.tags.title = e.title
        info.tags.artist = e.performer
        info.decoder = "CUE"
        t.info = info
        out.append(t)
    }
    return out
}

// MARK: - Info

/// Tags and audio parameters. Modules come from libopenmpt, everything else from TagLib.
func scanInfo(_ url: URL) -> AudioInfo? {
    if isModule(url) { return moduleInfo(url) }
    guard let file = taglib_file_new(url.path) else { return nil }
    defer { taglib_file_free(file) }
    guard taglib_file_is_valid(file) != 0 else { return nil }
    var info = AudioInfo()
    info.decoder = decoderName(url)
    if let props = taglib_file_audioproperties(file) {
        let secs = taglib_audioproperties_length(props)
        info.duration = secs > 0 ? Double(secs) : nil
        info.bitrate = Int(taglib_audioproperties_bitrate(props))
        info.sampleRate = Double(taglib_audioproperties_samplerate(props))
        info.channels = Int(taglib_audioproperties_channels(props))
    }
    if let tag = taglib_file_tag(file) {
        info.tags = readTags(tag)
    }
    // AVFoundation gives an exact length for most formats. Prefer it over TagLib's integer seconds.
    if let f = try? AVAudioFile(forReading: url), f.length > 0 {
        info.duration = Double(f.length) / f.processingFormat.sampleRate
    }
    return info
}

private func readTags(_ tag: UnsafePointer<TagLib_Tag>) -> Tags {
    func str(_ p: UnsafeMutablePointer<CChar>?) -> String? {
        guard let p else { return nil }
        let s = String(cString: p)
        return s.isEmpty ? nil : s
    }
    var t = Tags()
    t.title = str(taglib_tag_title(tag))
    t.artist = str(taglib_tag_artist(tag))
    t.album = str(taglib_tag_album(tag))
    let y = taglib_tag_year(tag)
    t.year = y > 0 ? String(y) : nil
    let n = taglib_tag_track(tag)
    t.track = n > 0 ? String(n) : nil
    t.comment = str(taglib_tag_comment(tag))
    taglib_tag_free_strings()
    return t
}

/// Writes the editable fields. Empty values clear the field.
func writeTags(_ url: URL, _ tags: Tags) -> Result<Void, Error> {
    if isModule(url) {
        return .failure(TagError.unsupported("tracker modules do not store editable tags"))
    }
    guard let file = taglib_file_new(url.path) else {
        return .failure(TagError.unsupported("cannot open \(url.lastPathComponent)"))
    }
    defer { taglib_file_free(file) }
    guard taglib_file_is_valid(file) != 0, let tag = taglib_file_tag(file) else {
        return .failure(TagError.unsupported("no writable tag in \(url.lastPathComponent)"))
    }
    func put(_ s: String?, _ set: (UnsafePointer<CChar>) -> Void) {
        (s ?? "").withCString { set($0) }
    }
    put(tags.title) { taglib_tag_set_title(tag, $0) }
    put(tags.artist) { taglib_tag_set_artist(tag, $0) }
    put(tags.album) { taglib_tag_set_album(tag, $0) }
    put(tags.comment) { taglib_tag_set_comment(tag, $0) }
    taglib_tag_set_year(tag, UInt32(tags.year.flatMap { UInt32($0) } ?? 0))
    taglib_tag_set_track(tag, UInt32(tags.track.flatMap { UInt32($0) } ?? 0))
    guard taglib_file_save(file) != 0 else {
        return .failure(TagError.unsupported("save failed for \(url.lastPathComponent)"))
    }
    return .success(())
}

enum TagError: Error, CustomStringConvertible {
    case unsupported(String)
    var description: String {
        switch self {
        case .unsupported(let s): return s
        }
    }
}

func decoderName(_ url: URL) -> String {
    if isModule(url) { return "Tracker" }
    switch url.pathExtension.lowercased() {
    case "mp1", "mp2", "mp3", "mpa", "mpga": return "MPEG"
    case "flac": return "FLAC"
    case "wav", "wave", "bwf", "aif", "aiff", "aifc": return "PCM"
    case "ogg", "oga": return "Vorbis"
    case "aac", "adts", "m4a", "m4b", "m4p", "m4r", "mp4": return "AAC"
    case "caf": return "CAF"
    case "mkv", "mka", "webm": return "Matroska"
    case "opus": return "Opus"
    default: return "Audio"
    }
}
