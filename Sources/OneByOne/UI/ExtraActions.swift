import AppKit

/// Intro scan, explore, external launchers, track log and periodic saves.
extension MainWindowController {
    @objc func startIntroScan() {
        if player.intro == nil { player.startIntro(perTrack: 10) } else { player.stopIntro() }
        refresh()
    }

    @objc func exploreFolder() {
        guard let folder = player.folder else { return }
        NSWorkspace.shared.open(folder)
    }

    /// Run external launcher `index` (0 to 2) on the heard track or the selected row.
    @objc func runExternal(_ sender: NSMenuItem) {
        let index = sender.tag
        let template = player.settings.external[index]
        guard !template.isEmpty else {
            statusLabel.stringValue = "External \(index + 1) is not set. Set it in Settings."
            return
        }
        let row = table.selectedRow ?? player.heard
        guard let row, player.tracks.indices.contains(row) else { return }
        let path = player.tracks[row].url.path
        let quoted = "'" + path.replacingOccurrences(of: "'", with: "'\\''") + "'"
        let command = template.replacingOccurrences(of: "%1", with: quoted)
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/bin/sh")
        p.arguments = ["-c", command]
        do {
            try p.run()
        } catch {
            statusLabel.stringValue = "Launch failed: \(error.localizedDescription)"
        }
    }

    @objc func showTrackLog() {
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 420),
                           styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        win.title = "Track Log"
        let scroll = NSScrollView(frame: win.contentView!.bounds)
        scroll.autoresizingMask = [.width, .height]
        scroll.hasVerticalScroller = true
        let text = NSTextView(frame: scroll.bounds)
        text.isEditable = false
        text.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        text.string = TrackLog.text().isEmpty ? "No tracks logged yet. Enable logging in Settings." : TrackLog.text()
        text.isVerticallyResizable = true
        text.textContainer?.widthTracksTextView = true
        scroll.documentView = text
        win.contentView?.addSubview(scroll)
        let controller = NSWindowController(window: win)
        logWindows.append(controller)
        controller.showWindow(nil)
    }

    /// Timer work done once per tick: intro scan and periodic resume save.
    func periodicWork() {
        tickCount += 1
        player.tickIntro()
        if tickCount % 12 == 0, player.refreshFolderIfChanged() {
            table.setTracks(player.folderTracks)
        }
        if player.isPlaying, tickCount % 20 == 0 { player.rememberResume() }
    }
}

extension MainWindowController {
    @objc func reloadFolder() {
        guard let folder = player.folder else { return }
        openFolder(folder)
    }

    @objc func toggleHTTP() {
        setRemote(remote == nil)
    }

    @objc func listModeChanged(_ sender: NSSegmentedControl) {
        player.settings.playlistFull = sender.selectedSegment == 1
        applySettings()
    }

    /// Show or hide the artwork panel in the sidebar.
    @objc func toggleArtwork() {
        player.settings.coverArt.toggle()
        player.save()
        applySettings()
    }

    @objc func togglePlaylistView() {
        player.settings.playlistOpen.toggle()
        player.save()
        applySettings()
    }

    @objc func toggleMute() {
        player.toggleMute()
    }

    @objc func volumeUp() { player.setVolume(player.settings.volume + 0.05) }
    @objc func volumeDown() { player.setVolume(player.settings.volume - 0.05) }

    @objc func quitAfterTrack() {
        player.quitAfterTrack = true
        statusLabel.stringValue = "Quit after this track"
    }

    @objc func removeFromPlaylist() {
        guard let rows = playlistTable.selectedRowsIfAny else { return }
        player.removePlaylistRows(rows)
    }
}

extension MainWindowController {
    @objc func focusSearch() {
        // Expands the collapsed toolbar search and puts the cursor in it.
        searchToolbarItem?.beginSearchInteraction()
    }
}

extension MainWindowController {
    /// Find Files: search names under the tree roots and play the pick.
    @objc func findFiles() {
        let w = FindFilesWindow(roots: tree.roots)
        w.onPick = { [weak self] url in
            guard let self else { return }
            let dir = url.deletingLastPathComponent()
            self.openFolder(dir)
            self.tree.select(dir)
            if let row = self.player.tracks.firstIndex(where: { $0.url == url }) {
                self.table.select(row: row)
                self.player.playFolder(row: row)
            }
        }
        findWindow = w
        w.showWindow(nil)
    }
}

extension MainWindowController {
    /// Title bar text, as the Title setting asks: name, elapsed, remaining, total.
    func windowTitle(for t: Track) -> String {
        let s = player.settings
        let name = t.display
        let pos = player.position
        let total = player.duration
        switch s.titleMode {
        case .name: break
        case .nameElapsed: return "\(clock(pos))  \(name)"
        case .nameRemaining:
            if let total { return "-\(clock(max(0, total - pos)))  \(name)" }
        case .nameElapsedTotal:
            if let total { return "\(clock(pos)) / \(clock(total))  \(name)" }
        }
        var line = "1by1 - \(name)"
        if s.upcomingInTitle, let row = player.heard {
            let next = nextIndex(count: player.tracks.count, cur: row, repeat: player.repeatMode)
            if let next, player.tracks.indices.contains(next) { line += "  next: \(player.tracks[next].display)" }
        }
        return line
    }
}

extension MainWindowController {
    /// Close hides the window when Minimize on Close is set.
    /// Remember the window size.
    func windowDidResize(_ notification: Notification) {
        guard sizeSaveEnabled, let win = window else { return }
        player.settings.winWidth = Int(win.frame.width)
        player.settings.winHeight = Int(win.frame.height)
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        // With the menu bar player on, closing keeps the app running there. Minimize is not used.
        guard !player.settings.trayIcon, player.settings.minimizeOnClose else { return true }
        sender.miniaturize(nil)
        return false
    }

    /// The table fades while the window is in the background, per Dim Inactive List.
    func windowDidResignKey(_ notification: Notification) { dimLists(true) }
    func windowDidBecomeKey(_ notification: Notification) { dimLists(false) }

    private func dimLists(_ inactive: Bool) {
        let dim = Double(player.settings.dimInactiveList) / 100
        table.view.alphaValue = inactive ? CGFloat(1 - dim) : 1
    }

    /// Settings-driven extras that have no layout of their own: tray icon, priority.
    /// Show a folder, select its first track and play it. Used by a double-click in the list or the tree.
    func playFolderFromStart(_ url: URL) {
        openFolder(url)
        tree.select(url)
        guard let first = player.folderTracks.firstIndex(where: { !$0.isFolder }) else { return }
        player.playFolder(row: first)
        table.select(row: first)
        table.scrollToRow(first)
    }

    /// Bring the main window back, from the menu bar or the Dock.
    @objc func showMainWindow() {
        showWindow(nil)
        window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applyExtras() {
        let s = player.settings
        if s.trayIcon, statusItem == nil {
            let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
            item.button?.image = NSImage(systemSymbolName: "music.note", accessibilityDescription: "1by1")
            let mini = MiniPlayer(controller: self, player: player)
            miniPlayer = mini
            let menu = NSMenu()
            menu.delegate = mini
            let host = NSMenuItem()
            host.view = mini.view
            menu.addItem(host)
            menu.addItem(.separator())
            menu.addItem(withTitle: "Show Main Window", action: #selector(showMainWindow), keyEquivalent: "").target = self
            menu.addItem(withTitle: "Quit 1by1", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "").target = NSApp
            item.menu = menu
            statusItem = item
        } else if !s.trayIcon, let item = statusItem {
            NSStatusBar.system.removeStatusItem(item)
            miniPlayer?.stop()
            miniPlayer = nil
            statusItem = nil
        }
        if s.highPriority, highPriorityToken == nil {
            highPriorityToken = ProcessInfo.processInfo.beginActivity(options: [.userInitiated, .latencyCritical],
                                                                      reason: "1by1 playback")
        } else if !s.highPriority, let t = highPriorityToken {
            ProcessInfo.processInfo.endActivity(t)
            highPriorityToken = nil
        }
    }
}
