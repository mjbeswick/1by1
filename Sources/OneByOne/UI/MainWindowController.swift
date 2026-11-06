import AppKit

/// Main window: folder tree on the left, track list and transport on the right.
final class MainWindowController: NSWindowController, NSToolbarDelegate, NSWindowDelegate {
    let player: PlayerController
    private let tree = FolderTreeController()
    private let table = TrackTableController(autosaveName: "TrackTable")
    private let playlistTable = TrackTableController(autosaveName: "PlaylistTable")
    private let seekSlider = NSSlider(value: 0, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let timeLabel = NSTextField(labelWithString: "0:00 / 0:00")
    private let volumeSlider = NSSlider(value: 1, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let statusLabel = NSTextField(labelWithString: "")
    private var splitController: NSSplitViewController?
    private var ticker: Timer?
    private weak var playPauseItem: NSToolbarItem?
    private var seeking = false
    private var infoWindows: [TrackInfoWindow] = []

    private enum ItemID {
        static let tree = NSToolbarItem.Identifier("tree")
        static let open = NSToolbarItem.Identifier("open")
        static let prev = NSToolbarItem.Identifier("prev")
        static let playPause = NSToolbarItem.Identifier("playPause")
        static let stop = NSToolbarItem.Identifier("stop")
        static let next = NSToolbarItem.Identifier("next")
        static let repeatMode = NSToolbarItem.Identifier("repeat")
        static let shuffle = NSToolbarItem.Identifier("shuffle")
        static let search = NSToolbarItem.Identifier("search")
    }

    init(player: PlayerController) {
        self.player = player
        let win = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1100, height: 680),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false)
        win.title = "1by1"
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
        table.onActivate = { [weak self] row in self?.player.playFolder(row: row) }
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
        let root = DropView()
        root.onDrop = { [weak self] urls in self?.handleDrop(urls) }

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

        playlistTable.onActivate = { [weak self] row in self?.player.playFromPlaylist(row: row) }
        let lists = NSSplitView()
        lists.isVertical = false
        lists.dividerStyle = .thin
        lists.addArrangedSubview(table.view)
        lists.addArrangedSubview(playlistTable.view)
        lists.setHoldingPriority(NSLayoutConstraint.Priority(250), forSubviewAt: 0)
        lists.translatesAutoresizingMaskIntoConstraints = false
        for v in [lists, transport, separator, statusLabel] as [NSView] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
        }
        playlistTable.view.heightAnchor.constraint(greaterThanOrEqualToConstant: 80).isActive = true

        NSLayoutConstraint.activate([
            lists.topAnchor.constraint(equalTo: root.topAnchor),
            lists.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            lists.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            lists.bottomAnchor.constraint(equalTo: transport.topAnchor),

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
         ItemID.repeatMode, ItemID.shuffle, .flexibleSpace, ItemID.search]
    }

    func toolbarDefaultItemIdentifiers(_ toolbar: NSToolbar) -> [NSToolbarItem.Identifier] {
        [ItemID.tree, ItemID.open, .flexibleSpace, ItemID.prev, ItemID.playPause, ItemID.stop, ItemID.next,
         ItemID.repeatMode, ItemID.shuffle, .flexibleSpace, ItemID.search]
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
        case ItemID.search:
            let field = NSSearchField(frame: NSRect(x: 0, y: 0, width: 180, height: 22))
            field.placeholderString = "Find in list"
            field.target = self
            field.action = #selector(searchFieldChanged(_:))
            item.label = "Find"
            item.view = field
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
        recordHistory(url)
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
        playlistTable.setTracks(player.playlist)
        table.setHeard(player.usesPlaylist ? nil : player.heard)
        playlistTable.setHeard(player.usesPlaylist ? player.heard : nil)
        let playing = player.isPlaying
        playPauseItem?.image = NSImage(systemSymbolName: playing ? "pause.fill" : "play.fill",
                                       accessibilityDescription: playing ? "Pause" : "Play")
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

// MARK: - Drop, search and go-to

extension MainWindowController {
    /// A dropped folder opens as the list. A dropped file opens its folder and plays the file.
    func handleDrop(_ urls: [URL]) {
        if let dir = urls.first(where: { isDirectory($0) }) {
            tree.add(root: dir)
            openFolder(dir)
            return
        }
        guard let first = urls.first(where: { isAudio($0) || isCueSheet($0) || isPlaylist($0) }) else { return }
        let dir = first.deletingLastPathComponent()
        openFolder(dir)
        if let row = player.tracks.firstIndex(where: { $0.url == first }) {
            table.select(row: row)
            player.play(row: row)
        }
    }

    func isDirectory(_ url: URL) -> Bool {
        var d: ObjCBool = false
        return FileManager.default.fileExists(atPath: url.path, isDirectory: &d) && d.boolValue
    }

    /// Select the first row whose name or tag title contains `text`.
    @objc func searchChanged(_ text: String) {
        guard !text.isEmpty else { return }
        if let row = player.tracks.firstIndex(where: { containsCI($0.display, text) || containsCI($0.name, text) }) {
            table.select(row: row)
        }
    }

    /// Ask for a 1-based row number and select it.
    @objc func goToRow() {
        let alert = NSAlert()
        alert.messageText = "Go to track"
        alert.informativeText = "Track number (1 to \(player.tracks.count))"
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 200, height: 24))
        alert.accessoryView = field
        alert.addButton(withTitle: "Go")
        alert.addButton(withTitle: "Cancel")
        guard alert.runModal() == .alertFirstButtonReturn,
              let n = Int(field.stringValue.trimmingCharacters(in: .whitespaces)),
              (1...max(1, player.tracks.count)).contains(n) else { return }
        table.select(row: n - 1)
    }
}

extension MainWindowController {
    @objc func searchFieldChanged(_ sender: NSSearchField) {
        searchChanged(sender.stringValue)
    }
}

extension MainWindowController {
    /// Track info for the selected row, or the track being heard.
    @objc func showTrackInfo() {
        let row = table.selectedRow ?? player.heard
        guard let row, player.tracks.indices.contains(row) else { return }
        let info = TrackInfoWindow(track: player.tracks[row])
        info.onSaved = { [weak self] _ in self?.table.reload() }
        infoWindows.append(info)
        info.showWindow(nil)
        info.window?.center()
    }
}

extension MainWindowController {
    /// Queue the selected folder rows at the end of the playlist.
    @objc func enqueueSelected() {
        let rows = Array(table.selectedRows)
        player.enqueue(rows: rows)
    }

    @objc func clearPlaylist() {
        player.clearPlaylist()
    }
}

// MARK: - Bookmarks and history

extension MainWindowController {
    /// Bookmark the current folder, or the heard track when one plays.
    @objc func addBookmark() {
        var s = player.settings
        if let t = player.heardTrack, !t.isCue {
            s.bookmarks.append(Settings.Bookmark(title: t.display, path: t.url.path, isFolder: false))
        } else if let folder = player.folder {
            s.bookmarks.append(Settings.Bookmark(title: folder.lastPathComponent, path: folder.path, isFolder: true))
        }
        s.bookmarks = Array(s.bookmarks.suffix(BookmarksMenu.maxBookmarks))
        player.settings = s
        player.save()
    }

    @objc func openBookmark(_ sender: NSMenuItem) {
        let bm = player.settings.bookmarks[sender.tag]
        let url = URL(fileURLWithPath: bm.path)
        if bm.isFolder {
            openFolder(url)
            tree.select(url)
        } else {
            let dir = url.deletingLastPathComponent()
            openFolder(dir)
            if let row = player.tracks.firstIndex(where: { $0.url == url }) {
                table.select(row: row)
                player.playFolder(row: row)
            }
        }
    }

    @objc func openHistory(_ sender: NSMenuItem) {
        let url = URL(fileURLWithPath: player.settings.history[sender.tag])
        openFolder(url)
        tree.select(url)
    }

    /// Called from openFolder: remember the folder at the top of history.
    func recordHistory(_ folder: URL) {
        var s = player.settings
        s.history.removeAll { $0 == folder.path }
        s.history.insert(folder.path, at: 0)
        s.history = Array(s.history.prefix(BookmarksMenu.maxHistory))
        player.settings = s
        player.save()
    }
}
