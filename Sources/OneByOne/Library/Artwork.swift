import AppKit
import AVFoundation

/// Embedded cover art read from a file's tags. AVFoundation reads ID3 (MP3), iTunes (M4A) and
/// picture blocks (FLAC). Cached per path, misses included. Main thread only.
private var embeddedCache: [String: NSImage?] = [:]

func embeddedArt(_ url: URL) -> NSImage? {
    if let hit = embeddedCache[url.path] { return hit }
    let asset = AVURLAsset(url: url)
    var found: NSImage?
    // Common metadata first, then every tag. Binary tags that do not decode as an image are skipped.
    for item in asset.commonMetadata + asset.metadata {
        if let data = item.dataValue, let img = NSImage(data: data), img.isValid {
            found = img
            break
        }
    }
    embeddedCache[url.path] = found
    return found
}

/// Art for a track: embedded first, then a cover file in its folder.
func albumArt(for url: URL, names: String) -> NSImage? {
    embeddedArt(url) ?? coverArt(in: url.deletingLastPathComponent(), names: names)
}
