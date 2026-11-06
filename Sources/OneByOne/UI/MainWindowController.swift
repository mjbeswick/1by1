import AppKit

/// Main window: folder tree on the left, track list and transport on the right.
final class MainWindowController: NSWindowController, NSToolbarDelegate, NSWindowDelegate {
    let player: PlayerController
    private let tree = FolderTreeController()
    private let table = TrackTableController()
    private let seekSlider = NSSlider(value: 0, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let timeLabel = NSTextField(labelWithString: "0:00 / 0:00")
    private let volumeSlider = NSSlider(value: 1, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let statusLabel = NSTextField(labelWithString: "")
    private var splitController: NSSplitViewController?
    private var ticker: Timer?
    private weak var playPauseItem: NSToolbarItem?
    private var seeking = false

    private enum ItemID {
        static let tree = NSToolbarItem.Identifier("tree")
        static let open = NSToolbarItem.Identifier("open")
        static let prev = NSToolbarItem.Identifier("prev")
        static let playPause = NSToolbarItem.Identifier("playPause")
        static let stop = NSToolbarItem.Identifier("stop")
        static let next = NSToolbarItem.Identifier("next")
        static let repeatMode = NSToolbarItem.Identifier("repeat")
        static let shuffle = NSToolbarItem.Identifier("shuffle")
    }

    init(player: PlayerController) {
        self.player = player
        let win = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1100, height: 680),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        win.title = "1by1"
        win.setFrameAutosaveName("MainWindow")
        win.toolbarStyle = .unified
        win.minSize = NSSize(width: 560, height: 340)
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

        let treeVC = NSViewController()
        treeVC.view = tree.view
        let contentVC = NSViewController()
        contentVC.view = makeContentView()

        let split = NSSplitViewController()
        let sidebar = NSSplitViewItem(sidebarWithViewController: treeVC)
        sidebar.minimumThickness = 160
        sidebar.maximumThickness = 420
        sidebar.canCollapse = true
        sidebar.isCollapsed = !player.settings.treeVisible
        let content = NSSplitViewItem(viewController: contentVC)
        content.minimumThickness = 320
        split.addSplitViewItem(sidebar)
        split.addSplitViewItem(content)
        splitController = split
        win.contentViewController = split
    }

    private func makeContentView() -> NSView {
        let root = NSView()

        seekSlider.target = self
        seekSlider.action = #selector(seekChanged(_:))
        seekSlider.isContinuous = true
        timeLabel.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
        timeLabel.textColor = .secondaryLabelColor
        volumeSlider.target = self
        volumeSlider.action = #selector(volumeChanged(_:))
        volumeSlider.doubleValue = Double(player.settings.volume)
        volumeSlider.controlSize = .small

        let speaker = NSImageView(image: NSImage(systemSymbolName: "speaker.wave.2", accessibilityDescription: "Volume") ?? NSImage())
        speaker.contentTintColor = .secondaryLabelColor

        let transport = NSStackView(views: [seekSlider, timeLabel, speaker, volumeSlider])
        transport.orientation = .horizontal
        transport.spacing = 8
        transport.edgeInsets = NSEdgeInsets(top: 6, left: 12, bottom: 6, right: 12)
        seekSlider.setContentHuggingPriority(.defaultLow, for: .horizontal)
        volumeSlider.widthAnchor.constraint(equalToConstant: 96).isActive = true

        statusLabel.font = .systemFont(ofSize: 11)
        statusLabel.textColor = .secondaryLabelColor
        statusLabel.lineBreakMode = .byTruncatingTail
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        let separator = NSBox()
        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false

        for v in [table.view, transport, separator, statusLabel] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
        }
        table.view.setContentHuggingPriority(.defaultLow, for: .vertical)

        NSLayoutConstraint.activate([
            table.view.topAnchor.constraint(equalTo: root.topAnchor),
            table.view.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            table.view.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            table.view.bottomAnchor.constraint(equalTo: transport.topAnchor),

            transport.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            transport.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            transport.bottomAnchor.constraint(equalTo: separator.topAnchor),

            separator.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            separator.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            separator.bottomAnchor.constraint(equalTo: statusLabel.topAnchor, constant: -3),

            statusLabel.leadingAnchor.constraint(equalTo: root.leadingAnchor, constant: 12),
            statusLabel.trailingAnchor.constraint(equalTo: root.trailingAnchor, constant: -12),
            statusLabel.bottomAnchor.constraint(equalTo: root.bottomAnchor, constant: -4),
        ])
        return root
    }

    private func buildToolbar(for win: NSWindow) {
        let tb = NSToolbar(identifier: "main")
        tb.delegate = self
        tb.displayMode = .iconOnly
        tb.allowsUserCustomization = false
        win.toolbar = tb
    }

    func toolbarAllowedItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [ItemID.tree, ItemID.open, ItemID.prev, ItemID.playPause, ItemID.stop, ItemID.next,
         ItemID.repeatMode, ItemID.shuffle, .flexibleSpace, .space]
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [ItemID.tree, ItemID.open, .flexibleSpace, ItemID.prev, ItemID.playPause, ItemID.stop, ItemID.next,
         ItemID.repeatMode, ItemID.shuffle]
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
        case ItemID.tree: button("sidebar.left", "Folder Tree", #selector(toggleTree))
        case ItemID.open: button("folder", "Open Folder", #selector(openFolderPanel))
        case ItemID.prev: button("backward.end.fill", "Previous", #selector(previousTrack))
        case ItemID.playPause:
            button("play.fill", "Play", #selector(playPausePressed))
            playPauseItem = item
        case ItemID.stop: button("stop.fill", "Stop", #selector(stopPressed))
        case ItemID.next: button("forward.end.fill", "Next", #selector(nextTrack))
        case ItemID.repeatMode: button("repeat", "Repeat", #selector(cycleRepeat))
        case ItemID.shuffle: button("shuffle", "Shuffle", #selector(toggleShuffle))
        default: return nil
        }
        return item
    }

    // MARK: - Wiring

    private func wirePlayer() {
        player.onChange = { [weak self] in self?.refresh() }
        player.onMessage = { [weak self] msg in self?.statusLabel.stringValue = msg }
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
        guard let item = splitController?.splitViewItems.first else { return }
        item.animator().isCollapsed.toggle()
        player.settings.treeVisible = item.isCollapsed == false
        player.save()
    }

    @objc func seekChanged(_ sender: NSSlider) {
        guard let d = player.duration, d > 0 else { return }
        let end = NSApp.currentEvent?.type == .leftMouseUp
        seeking = !end
        if end { player.seek(to: sender.doubleValue * d) }
    }

    @objc func volumeChanged(_ sender: NSSlider) {
        player.setVolume(Float(sender.doubleValue))
    }

    // MARK: - Refresh

    /// Repaint everything that depends on playback state.
    func refresh() {
        let playing = player.isPlaying
        playPauseItem?.image = NSImage(systemSymbolName: playing ? "pause.fill" : "play.fill",
                                       accessibilityDescription: playing ? "Pause" : "Play")
        table.setHeard(player.heard)
        if let t = player.heardTrack {
            window?.title = "1by1 - \(t.display)"
        }
        if player.isPaused { statusLabel.stringValue = "Paused" }
        updateStatus()
        tick()
    }

    private func tick() {
        guard let d = player.duration, d > 0 else {
            timeLabel.stringValue = clock(player.position) + " / " + (player.heard == nil ? "0:00" : "--:--")
            seekSlider.doubleValue = 0
            return
        }
        let pos = min(player.position, d)
        timeLabel.stringValue = "\(clock(pos)) / \(clock(d))"
        if !seeking { seekSlider.doubleValue = pos / d }
    }

    private func updateStatus() {
        guard player.heard == nil, !player.tracks.isEmpty else { return }
        let folder = player.folder?.lastPathComponent ?? ""
        statusLabel.stringValue = "\(player.tracks.count) tracks in \(folder)"
    }
}
