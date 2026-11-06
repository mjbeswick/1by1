import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    private(set) var player: PlayerController!
    private var window: MainWindowController!

    func applicationDidFinishLaunching(_ notification: Notification) {
        player = PlayerController(settings: Settings.load())
        window = MainWindowController(player: player)
        NSApp.mainMenu = MainMenu.build(target: self)
        window.showWindow(nil)
        window.window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationWillTerminate(_ notification: Notification) {
        player?.rememberResume()
        player?.save()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }

    // MARK: Menu actions routed through the responder chain's target

    @objc func showAbout(_ sender: Any?) {
        NSApp.orderFrontStandardAboutPanel(nil)
    }
}
