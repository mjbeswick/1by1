import AppKit

/// Main window: folder tree on the left, track list and transport on the right.
final class MainWindowController: NSWindowController, NSToolbarDelegate, NSWindowDelegate {
    let player: PlayerController
    private let split = NSSplitView()
    private let tree = FolderTreeController()
    private let table = TrackTableController()
    private let seek = NSSlider(value: 0, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let timeLabel = NSTextField(labelWithString: "0:00 / 0:00")
    private let volume = NSSlider(value: 1, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let status = NSTextField(labelWithString: "")
    private var ticker: Timer?
    private weak var playPauseItem: NSToolbarItem?
    private weak var treeItem: NSToolbarItem?
    private var seeking = false

    private enum ItemID {
        static let open = NSToolbarItem.Identifier("open")
        static let prev = NSToolbarItem.Identifier("prev")
        static let playPause = NSToolbarItem.Identifier("playPause")
        static let stop = NSToolbarItem.Identifier("stop")
        static let next = NSToolbarItem.Identifier("next")
        static let repeatMode = NSToolbarItem.Identifier("repeat")
        static let shuffle = NSToolbarItem.Identifier("shuffle")
        static let tree = NSToolbarItem.Identifier("tree")
        static let volume = NSToolbarItem.Identifier("volume")
    }

    init(player: PlayerController) {
        self.player = player
        let win = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 880, height: 560),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        win.title = "1by1"
        win.setFrameAutosaveName("MainWindow")
        win.titleVisibility = .visible
        win.toolbarStyle = .unified
        win.minSize = NSSize(width: 520, height: 320)
        super.init(window: win)
        win.delegate = self
        buildContent(in: win)
        buildToolbar(for: win)
        wirePlayer()
        ticker = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            self?.tick()
        }
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    // MARK: - Layout

   private func buildContent(in win: NSWindow) {
        tree.onSelect = { [weak self] url in self?.openFolder(url) }
        table.onActivate = { [weak self] row in self?.player.play(row: row) }
        table.onSelectionChange = { [weak self] in self?.updateStatus() }

        split.isVertical = true
        split.dividerStyle = .thin
        split.addArrangedSubview(tree.view)
        split.addArrangedSubview(makeRight())
        split.setHoldingPriority(NSLayoutConstraint.Priority(251), forSubviewAt: 0)
        tree.view.widthAnchor.constraint(greaterThanOrEqualToConstant: 160).isActive = true
        split.setPosition(220, ofDividerAt: 0)

        split.translatesAutoresizingMaskIntoConstraints = false
        win.contentView?.addSubview(split)
        NSLayoutConstraint.activate([
            split.leadingAnchor.constraint(equalTo: win.contentView!.leadingAnchor),
            split.trailingAnchor.constraint(equalTo: win.contentView!.trailingAnchor),
            split.topAnchor.constraint(equalTo: win.contentView!.topAnchor),
            split.bottomAnchor.constraint(equalTo: win.contentView!.bottomAnchor),
        ])
    }

   private func makeRight() -> NSView {
        let box = NSStackView()
        box.orientation = .vertical
        box.alignment = .leading
        box.spacing = 0
        box.edgeInsets = NSEdgeInsets(top: 0, left: 0, bottom: 0, right: 0)

        let bar = NSStackView()
        bar.orientation = .horizontal
        bar.spacing = 8
        bar.edgeInsets = NSEdgeInsets(top: 6, left: 12, bottom: 6, right: 12)
        seek.target = self
        seek.action = #selector(seekChanged(_:))
        seek.isContinuous = true
        seek.setContentHuggingPriority(.defaultLow, for: .horizontal)
        timeLabel.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
        timeLabel.textColor = .secondaryLabelColor
        volume.target = self
        volume.action = #selector(volumeChanged(_:))
        volume.widthAnchor.constraint(equalToConstant: 90).isActive = true
        volume.doubleValue = Double(player.settings.volume)
        bar.addArrangedSubview(seek)
        bar.addArrangedSubview(timeLabel)
        bar.addArrangedSubview(NSImageView(image: NSImage(systemSymbolName: "speaker.wave.2", accessibilityDescription: nil) ?? NSImage()))
        bar.addArrangedSubview(volume)

        status.font = .systemFont(ofSize: 11)
        status.textColor = .secondaryLabelColor
        status.lineBreakMode = .byTruncatingTail
        let statusBox = NSStackView(views: [status])
        statusBox.edgeInsets = NSEdgeInsets(top: 3, left: 12, bottom: 3, right: 12)

        box.addArrangedSubview(table.view)
        box.addArrangedSubview(bar)
        box.addArrangedSubview(NSBox.separator())
        box.addArrangedSubview(statusBox)
        table.view.setContentHuggingPriority(.defaultLow, for: .vertical)
        table.view.widthAnchor.constraint(equalTo: box.widthAnchor).isActive = true
        bar.widthAnchor.constraint(equalTo: box.widthAnchor).isActive = true
        return box
    }

   private func buildToolbar(for win: NSWindow) {
        let tb = NSToolbar(identifier: "main")
        tb.delegate = self
        tb.displayMode = .iconOnly
        tb.allowsUserCustomization = false
        win.toolbar = tb
    }

    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [ItemID.tree, ItemID.open, .flexibleSpace, ItemID.prev, ItemID.playPause, ItemID.stop, ItemID.next,
         ItemID.repeatMode, ItemID.shuffle, .space, ItemID.volume]
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [ItemID.tree, ItemID.open, .flexibleSpace, ItemID.prev, ItemID.playPause, ItemID.stop, ItemID.next,
         ItemID.repeatMode, ItemID.shuffle, .flexibleSpace]
    }

    func toolbar(_ toolbar: NSToolbar, itemForItemIdentifier id: NSToolbarItem.Identifier,
                 willBeInsertedIntoToolbar flag: Bool) -> NSToolbarItem? {
        let item = NSToolbarItem(itemIdentifier: id)
        func button(_ symbol: String, _ label: String, _ action: Selector) {
            item.label = label
            item.toolTip = label
            item.image = NSImage(systemSymbolName: symbol, accessibilityDescription: label)
            item.target = self
            item.action = action
        }
        switch id {
        case ItemID.open: button("folder", "Open Folder", #selector(openFolderPanel))
        case ItemID.prev: button("backward.end.fill", "Previous", #selector(previousTrack))
        case ItemID.playPause:
            button("play.fill", "Play", #selector(playPausePressed))
            playPauseItem = item
        case ItemID.stop: button("stop.fill", "Stop", #selector(stopPressed))
        case ItemID.next: button("forward.end.fill", "Next", #selector(nextTrack))
        case ItemID.repeatMode: button("repeat", "Repeat", #selector(cycleRepeat))
        case ItemID.shuffle: button("shuffle", "Shuffle", #selector(toggleShuffle))
        case ItemID.tree:
            button("sidebar.left", "Folder Tree", #selector(toggleTree))
            treeItem = item
        case ItemID.volume:
            item.label = "Volume"
            item.view = volume
        default: return nil
        }
        return item
    }

    // MARK: - Wiring

   private func wirePlayer() {
        player.onChange = { [weak self] in self?.refresh() }
        player.onMessage = { [weak self] msg in self?.status.stringValue = msg }
        tree.setRoots(initialRoots())
        if let last = player.settings.lastFolder.map(URL.init(fileURLWithPath:)),
           FileManager.default.fileExists(atPath: last.path) {
            openFolder(last)
            tree.select(last)
        } else if let first = tree.roots.first {
            openFolder(first)
        }
    }

   private func initialRoots() -> [URL] {
        if !player.settings.roots.isEmpty {
            return player.settings.roots.map { URL(fileURLWithPath: $0) }
        }
        let music = FileManager.default.urls(for: .musicDirectory, in: .userDomainMask).first
        return [music ?? FileManager.default.homeDirectoryForCurrentUser]
    }

    /// Load a folder: list it at once, then fill in tags on a background queue.
    func openFolder(_ url: URL) {
        let rows = folderTracks(url)
        player.setFolder(url, tracks: rows)
        table.setTracks(rows)
        if let resume = player.settings.folderResume[url.path],
           let idx = rows.firstIndex(where: { $0.url.path == resume.path }) {
            table.select(row: idx)
        }
        DispatchQueue.global(qos: .userInitiated).async {
            for t in rows where t.info == nil { t.info = scanInfo(t.url) }
            DispatchQueue.main.async { [weak self] in
                guard let self, self.player.folder == url else { return }
                self.table.reload()
                self.refresh()
            }
        }
        window?.title = "1by1 - \(url.lastPathComponent)"
        refresh()
    }

    // MARK: - Actions

    @objc func openFolderPanel() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        panel.prompt = "Open"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        tree.add(root: url)
        openFolder(url)
    }

    @objc func playPausePressed() {
        if player.engine.state == .stopped, table.selectedRow == nil, player.tracks.isEmpty { return }
        player.togglePause(selectedRow: table.selectedRow)
    }

    @objc func stopPressed() { player.stop() }
    @objc func nextTrack() { player.next() }
    @objc func previousTrack() { player.prev() }

    @objc func cycleRepeat() {
        player.repeatMode = player.repeatMode.next
        refresh()
    }

    @objc func toggleShuffle() {
        player.shuffle.toggle()
        refresh()
    }

    @objc func toggleTree() {
        let shown = !split.subviews[0].isHidden
        split.subviews[0].isHidden = shown
        player.settings.treeVisible = !shown
        player.save()
    }

    @objc func seekChanged(_ sender: NSSlider) {
        guard let d = player.duration, d > 0 else { return }
        seeking = true
        if NSApp.currentEvent?.type == .leftMouseUp { seeking = false }
        player.seek(to: sender.doubleValue * d)
    }

    @objc func volumeChanged(_ sender: NSSlider) {
        player.setVolume(Float(sender.doubleValue))
    }

    // MARK: - Refresh

    /// Repaint everything that depends on playback state.
    func refresh() {
        let playing = player.isPlaying
        let paused = player.isPaused
        playPauseItem?.image = NSImage(systemSymbolName: playing ? "pause.fill" : "play.fill",
                                       accessibilityDescription: playing ? "Pause" : "Play")
        table.setHeard(player.heard)
        if let t = player.heardTrack {
            window?.title = "1by1 - \(t.display)"
        }
        if paused { status.stringValue = "Paused" }
        updateStatus()
        tick()
    }

   private func tick() {
        guard let d = player.duration, d > 0 else {
            timeLabel.stringValue = clock(player.position) + " / " + (player.heard == nil ? "0:00" : "--:--")
            seek.doubleValue = 0
            return
        }
        let pos = min(player.position, d)
        timeLabel.stringValue = "\(clock(pos)) / \(clock(d))"
        if !seeking { seek.doubleValue = pos / d }
    }

   private func updateStatus() {
        if player.heard == nil, !player.tracks.isEmpty {
            let folder = player.folder?.lastPathComponent ?? ""
            status.stringValue = "\(player.tracks.count) tracks in \(folder)"
        }
    }
}

private extension NSBox {
    /// Hairline separator used between the transport bar and the status line.
    static func separator() -> NSBox {
        let b = NSBox()
        b.boxType = .separator
        return b
    }
}
