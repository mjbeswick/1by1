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

// MARK: - Move and folder operations

extension MainWindowController {
    /// Move tracks into a folder the user picks. Name clashes are skipped, never overwritten.
    func moveTracks(_ tracks: [Track]) {
        guard !tracks.isEmpty, let window else { return }
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.prompt = "Move Here"
        panel.message = "Move \(tracks.count) track(s) to:"
        panel.beginSheetModal(for: window) { [weak self] response in
            guard response == .OK, let dest = panel.url, let self else { return }
            var moved = Set<URL>()
            var skipped = 0
            for t in tracks {
                let target = dest.appendingPathComponent(t.url.lastPathComponent)
                guard !FileManager.default.fileExists(atPath: target.path) else { skipped += 1; continue }
                do {
                    try FileManager.default.moveItem(at: t.url, to: target)
                    moved.insert(t.url)
                } catch {
                    skipped += 1
                }
            }
            let heardMoved = self.player.heardTrack.map { moved.contains($0.url) } ?? false
            self.reloadFolder()
            self.player.prunePlaylist()
            self.refresh()
            if heardMoved { self.player.next() }
            self.statusLabel.stringValue = "Moved \(moved.count)" + (skipped > 0 ? ", \(skipped) skipped" : "")
        }
    }

    /// Ask for a name in a sheet. Calls `done` with the trimmed text, or nil on cancel.
    private func askName(title: String, value: String, done: @escaping (String?) -> Void) {
        guard let window else { return }
        let alert = NSAlert()
        alert.messageText = title
        let field = NSTextField(string: value)
        field.frame = NSRect(x: 0, y: 0, width: 260, height: 24)
        alert.accessoryView = field
        alert.addButton(withTitle: "OK")
        alert.addButton(withTitle: "Cancel")
        alert.window.initialFirstResponder = field
        alert.beginSheetModal(for: window) { response in
            let text = field.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
            done(response == .alertFirstButtonReturn && !text.isEmpty && !text.contains("/") ? text : nil)
        }
    }

    /// Create a folder inside `parent`.
    func newFolder(in parent: URL) {
        askName(title: "New folder name", value: "New Folder") { [weak self] name in
            guard let name, let self else { return }
            let url = parent.appendingPathComponent(name)
            do {
                try FileManager.default.createDirectory(at: url, withIntermediateDirectories: false)
                self.tree.setRoots(self.tree.roots)
                self.tree.select(url)
            } catch {
                self.statusLabel.stringValue = "New folder failed: \(error.localizedDescription)"
            }
        }
    }

    /// Rename a folder in place. Refreshes the list when it is the open folder.
    func renameFolder(_ url: URL) {
        askName(title: "Rename folder", value: url.lastPathComponent) { [weak self] name in
            guard let name, let self, name != url.lastPathComponent else { return }
            let target = url.deletingLastPathComponent().appendingPathComponent(name)
            do {
                try FileManager.default.moveItem(at: url, to: target)
                let wasOpen = self.player.folder == url
                self.tree.setRoots(self.tree.roots)
                self.tree.select(target)
                if wasOpen { self.openFolder(target) }
            } catch {
                self.statusLabel.stringValue = "Rename failed: \(error.localizedDescription)"
            }
        }
    }

    /// Move a folder to the Trash after a confirmation. Tracks inside go with it.
    func trashFolder(_ url: URL) {
        guard let window else { return }
        let alert = NSAlert()
        alert.messageText = "Move \"\(url.lastPathComponent)\" to the Trash?"
        alert.informativeText = "The folder and everything in it go to the Trash."
        alert.addButton(withTitle: "Move to Trash")
        alert.addButton(withTitle: "Cancel")
        alert.beginSheetModal(for: window) { [weak self] response in
            guard response == .alertFirstButtonReturn, let self else { return }
            let wasOpen = self.player.folder == url
            let parent = url.deletingLastPathComponent()
            NSWorkspace.shared.recycle([url]) { _, error in
                DispatchQueue.main.async {
                    if let error {
                        self.statusLabel.stringValue = "Trash failed: \(error.localizedDescription)"
                        return
                    }
                    self.tree.setRoots(self.tree.roots)
                    self.tree.select(parent)
                    if wasOpen { self.openFolder(parent) }
                }
            }
        }
    }
}
