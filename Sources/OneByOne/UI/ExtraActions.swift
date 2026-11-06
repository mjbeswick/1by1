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
        player.tickIntro()
        if player.isPlaying {
            tickCount += 1
            if tickCount % 20 == 0 { player.rememberResume() }
        }
    }
}
