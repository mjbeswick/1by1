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

    /// Compact view: the window keeps only the transport line.
    @objc func toggleCompact() {
        player.settings.compact.toggle()
        player.save()
        applyCompact()
    }

    func applyCompact() {
        let compact = player.settings.compact
        splitController?.view.isHidden = compact
        guard let win = window else { return }
        var frame = win.frame
        let height: CGFloat = compact ? 0 : CGFloat(player.settings.winHeight)
        let delta = height - frame.height
        frame.size.height = compact ? 40 : max(340, height)
        frame.origin.y -= (compact ? (frame.height - 40) : delta)
        win.setFrame(frame, display: true, animate: false)
    }

    @objc func toggleHTTP() {
        setRemote(remote == nil)
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
        window?.makeFirstResponder(searchField)
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
