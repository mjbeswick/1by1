import AppKit

/// Keys from the Rust build that menus cannot express: bare arrows, Delete, Insert and Pause.
/// Modifier shortcuts live in the menu bar (see MainMenu).
extension MainWindowController {
    func installKeyMonitor() {
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self else { return event }
            return self.handleKey(event) ? nil : event
        }
    }

    /// Returns true when the key was handled.
    private func handleKey(_ e: NSEvent) -> Bool {
        // Text fields and the search box keep their keys.
        if window?.firstResponder is NSText { return false }
        let mods = e.modifierFlags.intersection([.command, .control, .option, .shift])
        let s = player.settings
        switch (e.keyCode, mods) {
        case (53, []) where s.escQuit:                                           // Escape quits when set
            NSApp.terminate(nil)
            return true
        case (36, [.option]):                                                    // Alt-Return: track info
            showTrackInfo()
            return true
        case (97, []):                                                           // F6: settings
            showSettings()
            return true
        case (123, []):  player.rewind(by: -Double(s.rewindSecs)); return true   // Left
        case (124, []):  player.rewind(by: Double(s.forwardSecs)); return true   // Right
        case (123, [.shift]): stepFolder(-1); return true
        case (124, [.shift]): stepFolder(1); return true
        case (126, [.control]): player.setVolume(s.volume + (s.fineVolume ? 0.01 : 0.05)); return true
        case (125, [.control]): player.setVolume(s.volume - (s.fineVolume ? 0.01 : 0.05)); return true
        case (126, [.option]) where window?.firstResponder === playlistTable.table:
            if let rows = playlistTable.selectedRowsIfAny { player.movePlaylistRows(rows, by: -1) }
            return true
        case (125, [.option]) where window?.firstResponder === playlistTable.table:
            if let rows = playlistTable.selectedRowsIfAny { player.movePlaylistRows(rows, by: 1) }
            return true
        case (51, []) where window?.firstResponder === playlistTable.table:       // Delete in playlist
            removeFromPlaylist()
            return true
        case (117, []), (114, []):                                               // Delete, Insert: skip to next
            player.next()
            return true
        default:
            return false
        }
    }
}
