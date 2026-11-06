import AppKit

/// Rebuilds the Bookmarks and Recent Folders submenus each time they open.
final class BookmarksMenu: NSObject, NSMenuDelegate {
    static let maxBookmarks = 30
    static let maxHistory = 10
    let bookmarks: NSMenu
    let history: NSMenu
    private let settingsProvider: () -> Settings

    init(settings: @escaping () -> Settings) {
        bookmarks = NSMenu(title: "Bookmarks")
        history = NSMenu(title: "Recent Folders")
        settingsProvider = settings
        super.init()
        bookmarks.delegate = self
        history.delegate = self
    }

    func menuNeedsUpdate(_ menu: NSMenu) {
        menu.removeAllItems()
        let s = settingsProvider()
        if menu === bookmarks {
            menu.addItem(withTitle: "Add Bookmark", action: #selector(MainWindowController.addBookmark), keyEquivalent: "d")
            menu.addItem(.separator())
            for (i, b) in s.bookmarks.enumerated() {
                let item = NSMenuItem(title: b.title, action: #selector(MainWindowController.openBookmark(_:)), keyEquivalent: "")
                item.tag = i
                menu.addItem(item)
            }
        } else {
            for (i, path) in s.recentDirs.prefix(s.historyEntries).enumerated() {
                let item = NSMenuItem(title: URL(fileURLWithPath: path).lastPathComponent,
                                      action: #selector(MainWindowController.openHistory(_:)), keyEquivalent: "")
                item.tag = i
                item.toolTip = path
                menu.addItem(item)
            }
            if s.recentDirs.isEmpty {
                menu.addItem(withTitle: "No Recent Folders", action: nil, keyEquivalent: "").isEnabled = false
            }
        }
    }
}
