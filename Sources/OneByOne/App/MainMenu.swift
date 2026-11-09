import AppKit

/// Native menu bar. Items send actions to the first responder or to the window controller.
enum MainMenu {
    static func build(target: AppDelegate) -> NSMenu {
        let bar = NSMenu()

        let app = NSMenu()
        app.addItem(withTitle: "About 1by1", action: #selector(AppDelegate.showAbout(_:)), keyEquivalent: "")
            .target = target
        app.addItem(.separator())
        app.addItem(withTitle: "Hide 1by1", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
        app.addItem(withTitle: "Quit 1by1", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        bar.addItem(submenu: app, title: "1by1")

        let file = NSMenu(title: "File")
        file.addItem(withTitle: "Open Folder...", action: #selector(MainWindowController.openFolderPanel), keyEquivalent: "o")
        file.addItem(withTitle: "Load Playlist...", action: #selector(MainWindowController.loadPlaylist), keyEquivalent: "")
        file.addItem(withTitle: "Save Playlist...", action: #selector(MainWindowController.savePlaylist), keyEquivalent: "")
        bar.addItem(submenu: file, title: "File")

        let play = NSMenu(title: "Play")
        play.addItem(withTitle: "Play / Pause", action: #selector(MainWindowController.playPausePressed), keyEquivalent: " ")
        play.addItem(withTitle: "Stop", action: #selector(MainWindowController.stopPressed), keyEquivalent: ".")
        play.addItem(.separator())
        play.addItem(withTitle: "Previous Track", action: #selector(MainWindowController.previousTrack), keyEquivalent: functionKey(NSLeftArrowFunctionKey))
            .keyEquivalentModifierMask = [.control]
        play.addItem(withTitle: "Next Track", action: #selector(MainWindowController.nextTrack), keyEquivalent: functionKey(NSRightArrowFunctionKey))
            .keyEquivalentModifierMask = [.control]
        play.addItem(.separator())
        play.addItem(withTitle: "Go to Track...", action: #selector(MainWindowController.goToRow), keyEquivalent: "g")
        play.addItem(withTitle: "Track Info...", action: #selector(MainWindowController.showTrackInfo), keyEquivalent: "i")
        play.addItem(withTitle: "Add to Playlist", action: #selector(MainWindowController.enqueueSelected), keyEquivalent: "\r")
            .keyEquivalentModifierMask = [.control]
        play.addItem(withTitle: "Clear Playlist", action: #selector(MainWindowController.clearPlaylist), keyEquivalent: "")
        play.addItem(.separator())
        let intro = play.addItem(withTitle: "Intro Scan", action: #selector(MainWindowController.startIntroScan), keyEquivalent: "n")
        intro.keyEquivalentModifierMask = [.control]
        let explore = play.addItem(withTitle: "Explore Folder", action: #selector(MainWindowController.exploreFolder), keyEquivalent: "x")
        explore.keyEquivalentModifierMask = [.control]
        for i in 0..<3 {
            let item = play.addItem(withTitle: "External \(i + 1)", action: #selector(MainWindowController.runExternal(_:)), keyEquivalent: "")
            item.tag = i
        }
        play.addItem(.separator())
        let log = play.addItem(withTitle: "Track Log", action: #selector(MainWindowController.showTrackLog), keyEquivalent: "l")
        log.keyEquivalentModifierMask = [.command, .shift]
        play.addItem(.separator())
        play.addItem(withTitle: "Mute", action: #selector(MainWindowController.toggleMute), keyEquivalent: "m")
            .keyEquivalentModifierMask = [.control]
        let quitAfter = play.addItem(withTitle: "Quit After This Track", action: #selector(MainWindowController.quitAfterTrack), keyEquivalent: "Q")
        quitAfter.keyEquivalentModifierMask = [.control, .shift]
        let find = file.addItem(withTitle: "Find Files...", action: #selector(MainWindowController.findFiles), keyEquivalent: functionKey(NSF3FunctionKey))
        find.keyEquivalentModifierMask = []
        let focus = play.addItem(withTitle: "Find in List", action: #selector(MainWindowController.focusSearch), keyEquivalent: "f")
        focus.keyEquivalentModifierMask = [.control]
        play.addItem(withTitle: "Repeat", action: #selector(MainWindowController.cycleRepeat), keyEquivalent: "r")
            .keyEquivalentModifierMask = [.control]
        play.addItem(withTitle: "Shuffle", action: #selector(MainWindowController.toggleShuffle), keyEquivalent: "p")
            .keyEquivalentModifierMask = [.control]
        bar.addItem(submenu: play, title: "Play")

        let view = NSMenu(title: "View")
        view.addItem(withTitle: "Show Folder Tree", action: #selector(MainWindowController.toggleTree), keyEquivalent: functionKey(NSF4FunctionKey))
        view.addItem(.separator())
        view.addItem(withTitle: "Compact View", action: #selector(MainWindowController.toggleCompact), keyEquivalent: functionKey(NSF7FunctionKey))
        view.addItem(withTitle: "Playlist View", action: #selector(MainWindowController.togglePlaylistView), keyEquivalent: functionKey(NSF5FunctionKey))
            .keyEquivalentModifierMask = [.shift]
        view.addItem(withTitle: "Reload Folder", action: #selector(MainWindowController.reloadFolder), keyEquivalent: functionKey(NSF5FunctionKey))
        bar.addItem(submenu: view, title: "View")

        let remoteItem = NSMenuItem(title: "Start HTTP Remote", action: #selector(MainWindowController.toggleRemote), keyEquivalent: "R")
        remoteItem.tag = 900
        remoteItem.keyEquivalentModifierMask = [.control, .shift]
        remoteItem.keyEquivalent = "W"
        file.addItem(remoteItem)
        let bm = BookmarksMenu(settings: { Settings.load() })
        let bookmarksItem = NSMenuItem(title: "Bookmarks", action: nil, keyEquivalent: "")
        bookmarksItem.submenu = bm.bookmarks
        bar.addItem(bookmarksItem)
        let viewItem = NSMenuItem(title: "Big Title View", action: #selector(MainWindowController.toggleBigTitle), keyEquivalent: "b")
        viewItem.keyEquivalentModifierMask = [.control]
        view.addItem(viewItem)
        let prefs = NSMenuItem(title: "Settings...", action: #selector(MainWindowController.showSettings), keyEquivalent: ",")
        app.insertItem(prefs, at: 2)
        app.insertItem(.separator(), at: 3)
        let historyItem = NSMenuItem(title: "History", action: nil, keyEquivalent: "")
        historyItem.submenu = bm.history
        file.addItem(historyItem)

        let window = NSMenu(title: "Window")
        window.addItem(withTitle: "Minimize", action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
        window.addItem(withTitle: "Close", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        bar.addItem(submenu: window, title: "Window")
        NSApp.windowsMenu = window

        return bar
    }
}

private extension NSMenu {
    @discardableResult
    func addItem(withTitle title: String, action: Selector?, keyEquivalent: String) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: keyEquivalent)
        addItem(item)
        return item
    }

    func addItem(submenu: NSMenu, title: String) {
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.submenu = submenu
        addItem(item)
    }
}

/// Key equivalent string for an AppKit function key code.
private func functionKey(_ code: Int) -> String {
    String(UnicodeScalar(UInt32(code)).map(Character.init) ?? " ")
}
