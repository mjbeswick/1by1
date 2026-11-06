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
        play.addItem(withTitle: "Repeat", action: #selector(MainWindowController.cycleRepeat), keyEquivalent: "r")
            .keyEquivalentModifierMask = [.control]
        play.addItem(withTitle: "Shuffle", action: #selector(MainWindowController.toggleShuffle), keyEquivalent: "p")
            .keyEquivalentModifierMask = [.control]
        bar.addItem(submenu: play, title: "Play")

        let view = NSMenu(title: "View")
        view.addItem(withTitle: "Show Folder Tree", action: #selector(MainWindowController.toggleTree), keyEquivalent: functionKey(NSF4FunctionKey))
        bar.addItem(submenu: view, title: "View")

        let window = NSMenu(title: "Window")
        window.addItem(withTitle: "Minimize", action: #selector(NSWindow.performMiniaturize(_:)), keyEquivalent: "m")
        window.addItem(withTitle: "Close", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        bar.addItem(submenu: window, title: "Window")
        NSApp.windowsMenu = window

        return bar
    }
}

private extension NSMenu {
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
