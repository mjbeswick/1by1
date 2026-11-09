import AppKit

/// Table that offers a menu for the row under the pointer. The row is selected first.
final class ContextTableView: NSTableView {
    var menuForRow: ((Int) -> NSMenu?)?
    override func menu(for event: NSEvent) -> NSMenu? {
        let r = row(at: convert(event.locationInWindow, from: nil))
        guard r >= 0 else { return nil }
        if !isRowSelected(r) { selectRowIndexes(IndexSet(integer: r), byExtendingSelection: false) }
        return menuForRow?(r)
    }
}

/// Folder tree with the same right-click behaviour.
final class ContextOutlineView: NSOutlineView {
    var menuForRow: ((Int) -> NSMenu?)?
    override func menu(for event: NSEvent) -> NSMenu? {
        let r = row(at: convert(event.locationInWindow, from: nil))
        guard r >= 0 else { return nil }
        if !isRowSelected(r) { selectRowIndexes(IndexSet(integer: r), byExtendingSelection: false) }
        return menuForRow?(r)
    }
}

/// Runs a closure from a menu item. The item keeps this object alive through `representedObject`.
private final class MenuAction: NSObject {
    let run: () -> Void
    init(_ run: @escaping () -> Void) { self.run = run }
    @objc func fire() { run() }
}

private func menuItem(_ title: String, symbol: String? = nil, _ run: @escaping () -> Void) -> NSMenuItem {
    let action = MenuAction(run)
    let item = NSMenuItem(title: title, action: #selector(MenuAction.fire), keyEquivalent: "")
    item.target = action
    item.representedObject = action
    if let symbol { item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: title) }
    return item
}

private func copyText(_ text: String) {
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(text, forType: .string)
}

extension MainWindowController {
    /// Menu for a track list row. The playlist uses its own actions.
    func trackMenu(row: Int, tracks: [Track], inPlaylist: Bool) -> NSMenu? {
        guard tracks.indices.contains(row) else { return nil }
        let t = tracks[row]
        let menu = NSMenu()
        if inPlaylist {
            menu.addItem(menuItem("Play", symbol: "play.fill") { [weak self] in self?.player.playFromPlaylist(row: row) })
            menu.addItem(menuItem("Remove from Playlist", symbol: "minus.circle") { [weak self] in self?.removeFromPlaylist() })
        } else {
            menu.addItem(menuItem(t.isFolder ? "Open Folder" : "Play", symbol: t.isFolder ? "folder" : "play.fill") { [weak self] in self?.table.onActivate?(row) })
            if !t.isFolder {
                menu.addItem(menuItem("Add to Playlist", symbol: "text.badge.plus") { [weak self] in self?.enqueueSelected() })
                menu.addItem(menuItem("Track Info...", symbol: "info.circle") { [weak self] in self?.showTrackInfo() })
            }
        }
        if !t.isFolder && !inPlaylist {
            menu.addItem(menuItem("Move to Folder...", symbol: "folder.badge.gearshape") { [weak self] in
                guard let self else { return }
                let rows = self.table.selectedRows.filter { self.player.tracks.indices.contains($0) && !self.player.tracks[$0].isFolder }
                self.moveTracks(rows.map { self.player.tracks[$0] })
            })
            menu.addItem(menuItem("Move to Trash", symbol: "trash") { [weak self] in self?.trashSelected() })
        }
        if !t.isFolder {
            let rate = NSMenuItem(title: "Rating", action: nil, keyEquivalent: "")
            rate.image = NSImage(systemSymbolName: "star", accessibilityDescription: "Rating")
            rate.submenu = ratingMenu(for: t.url)
            menu.addItem(rate)
        }
        menu.addItem(.separator())
        menu.addItem(menuItem("Show in Finder", symbol: "magnifyingglass") { NSWorkspace.shared.activateFileViewerSelecting([t.url]) })
        menu.addItem(menuItem("Copy Path", symbol: "doc.on.doc") { copyText(t.url.path) })
        return menu
    }

    /// Rating choices for one file. Stars or thumbs, as set in Settings.
    func ratingMenu(for url: URL) -> NSMenu {
        let menu = NSMenu()
        func rate(_ title: String, _ value: Int) {
            menu.addItem(menuItem(title) { [weak self] in
                Ratings.shared.set(url, value)
                self?.table.reload()
                self?.playlistTable.reload()
            })
        }
        switch ratingStyleSetting {
        case .stars:
            rate("None", 0)
            for v in 1...5 { rate(String(repeating: "\u{2605}", count: v), v) }
        case .thumbs:
            rate("Thumbs Up", 5)
            rate("Thumbs Down", 1)
            rate("None", 0)
        }
        return menu
    }

    /// Menu for a folder tree row.
    func treeMenu(url: URL) -> NSMenu {
        let menu = NSMenu()
        menu.addItem(menuItem("Open Folder", symbol: "folder") { [weak self] in self?.openFolder(url) })
        menu.addItem(menuItem("New Folder...", symbol: "folder.badge.plus") { [weak self] in self?.newFolder(in: url) })
        menu.addItem(menuItem("Rename Folder...", symbol: "pencil") { [weak self] in self?.renameFolder(url) })
        menu.addItem(menuItem("Move Folder to Trash", symbol: "trash") { [weak self] in self?.trashFolder(url) })
        menu.addItem(.separator())
        menu.addItem(menuItem("Show in Finder", symbol: "magnifyingglass") { NSWorkspace.shared.activateFileViewerSelecting([url]) })
        menu.addItem(menuItem("Copy Path", symbol: "doc.on.doc") { copyText(url.path) })
        return menu
    }
}
