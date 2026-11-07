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

private func menuItem(_ title: String, _ run: @escaping () -> Void) -> NSMenuItem {
    let action = MenuAction(run)
    let item = NSMenuItem(title: title, action: #selector(MenuAction.fire), keyEquivalent: "")
    item.target = action
    item.representedObject = action
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
            menu.addItem(menuItem("Play") { [weak self] in self?.player.playFromPlaylist(row: row) })
            menu.addItem(menuItem("Remove from Playlist") { [weak self] in self?.removeFromPlaylist() })
        } else {
            menu.addItem(menuItem(t.isFolder ? "Open Folder" : "Play") { [weak self] in self?.table.onActivate?(row) })
            if !t.isFolder {
                menu.addItem(menuItem("Add to Playlist") { [weak self] in self?.enqueueSelected() })
                menu.addItem(menuItem("Track Info...") { [weak self] in self?.showTrackInfo() })
            }
        }
        menu.addItem(.separator())
        menu.addItem(menuItem("Show in Finder") { NSWorkspace.shared.activateFileViewerSelecting([t.url]) })
        menu.addItem(menuItem("Copy Path") { copyText(t.url.path) })
        return menu
    }

    /// Menu for a folder tree row.
    func treeMenu(url: URL) -> NSMenu {
        let menu = NSMenu()
        menu.addItem(menuItem("Open Folder") { [weak self] in self?.openFolder(url) })
        menu.addItem(.separator())
        menu.addItem(menuItem("Show in Finder") { NSWorkspace.shared.activateFileViewerSelecting([url]) })
        menu.addItem(menuItem("Copy Path") { copyText(url.path) })
        return menu
    }
}
