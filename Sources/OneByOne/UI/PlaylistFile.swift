import AppKit
import UniformTypeIdentifiers

extension MainWindowController {
    /// Command-Z: step the playlist back one change.
    @objc func undoPlaylistChange() {
        player.undoPlaylist()
        refresh()
        statusLabel.stringValue = "Playlist change undone"
    }

    /// Shift-Command-Z: take back one undo.
    @objc func redoPlaylistChange() {
        player.redoPlaylist()
        refresh()
        statusLabel.stringValue = "Playlist change redone"
    }

    /// Write the playlist as an extended M3U (UTF-8). Cue sheet bounds are not kept.
    @objc func savePlaylist() {
        guard let window else { return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "m3u8") ?? .plainText]
        panel.nameFieldStringValue = "playlist.m3u8"
        panel.beginSheetModal(for: window) { [weak self] response in
            guard response == .OK, let url = panel.url, let self else { return }
            var lines = ["#EXTM3U"]
            for t in self.player.playlist {
                lines.append("#EXTINF:\(Int((t.duration ?? -1).rounded())),\(t.display)")
                lines.append(t.url.path)
            }
            do {
                try (lines.joined(separator: "\n") + "\n").write(to: url, atomically: true, encoding: .utf8)
                self.statusLabel.stringValue = "Saved \(self.player.playlist.count) tracks to \(url.lastPathComponent)"
            } catch {
                self.statusLabel.stringValue = "Save failed: \(error.localizedDescription)"
            }
        }
    }

    /// Append the tracks of an M3U or M3U8 file to the playlist.
    @objc func loadPlaylist() {
        guard let window else { return }
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "m3u8") ?? .plainText,
                                     UTType(filenameExtension: "m3u") ?? .plainText]
        panel.allowsMultipleSelection = false
        panel.beginSheetModal(for: window) { [weak self] response in
            guard response == .OK, let url = panel.url, let self else { return }
            let tracks = readPlaylist(url).filter { FileManager.default.fileExists(atPath: $0.path) }.map(Track.init(url:))
            self.player.playlist += tracks
            self.refresh()
            self.statusLabel.stringValue = "Loaded \(tracks.count) tracks from \(url.lastPathComponent)"
        }
    }

    /// Queue every audio file under `root`, subfolders included. The scan runs off the main thread.
    func addTreeToPlaylist(_ root: URL) {
        statusLabel.stringValue = "Scanning \(root.lastPathComponent)..."
        DispatchQueue.global(qos: .userInitiated).async {
            var found: [URL] = []
            if let e = FileManager.default.enumerator(at: root,
                                                      includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey],
                                                      options: [.skipsHiddenFiles]) {
                for case let url as URL in e {
                    let v = try? url.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])
                    if v?.isSymbolicLink == true { e.skipDescendants(); continue }
                    if v?.isDirectory == false, isAudio(url) { found.append(url) }
                }
            }
            // Natural order by path, so each folder's files keep their sequence.
            found.sort { naturalCompare($0.path, $1.path) == .orderedAscending }
            let tracks = found.map(Track.init(url:))
            DispatchQueue.main.async { [weak self] in
                guard let self else { return }
                self.player.playlist += tracks
                self.refresh()
                self.statusLabel.stringValue = "Added \(tracks.count) tracks from \(root.lastPathComponent)"
            }
        }
    }
}
