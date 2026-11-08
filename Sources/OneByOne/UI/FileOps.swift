import AppKit

extension MainWindowController {
    /// Option-Delete: move the selected tracks to the Trash after a confirmation. Folders are skipped.
    @objc func trashSelected() {
        let tracks = table.selectedRows.compactMap { row -> Track? in
            guard player.tracks.indices.contains(row), !player.tracks[row].isFolder else { return nil }
            return player.tracks[row]
        }
        guard !tracks.isEmpty, let window else { return }
        let alert = NSAlert()
        alert.messageText = tracks.count == 1
            ? "Move \"\(tracks[0].name)\" to the Trash?"
            : "Move \(tracks.count) tracks to the Trash?"
        alert.informativeText = "You can restore them from the Trash."
        alert.addButton(withTitle: "Move to Trash")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: window) { [weak self] response in
            guard response == .alertFirstButtonReturn, let self else { return }
            let trashed = Set(tracks.map { $0.url })
            let heardTrashed = self.player.heardTrack.map { trashed.contains($0.url) } ?? false
            NSWorkspace.shared.recycle(tracks.map { $0.url }) { _, error in
                DispatchQueue.main.async {
                    if let error {
                        self.statusLabel.stringValue = "Trash failed: \(error.localizedDescription)"
                        return
                    }
                    self.reloadFolder()
                    self.player.prunePlaylist()
                    self.refresh()
                    if heardTrashed { self.player.next() }
                }
            }
        }
    }
}
