import AppKit

/// Main window: folder tree on the left, track list and transport on the right.
final class MainWindowController: NSWindowController, NSToolbarDelegate, NSWindowDelegate {
    let player: PlayerController
    let tree = FolderTreeController()
    let table = TrackTableController(autosaveName: "TrackTable")
    let playlistTable = TrackTableController(autosaveName: "PlaylistTable")
    let seekSlider = NSSlider(value: 0, minValue: 0, maxValue: 1, target: nil, action: nil)
    let timeLabel = NSTextField(labelWithString: "0:00 / 0:00")
    let volumeSlider = NSSlider(value: 1, minValue: 0, maxValue: 1, target: nil, action: nil)
    let statusLabel = NSTextField(labelWithString: "")
    var splitController: NSSplitViewController?
    var ticker: Timer?
    private weak var playPauseItem: NSToolbarItem?
    var seeking = false
    var infoWindows: [TrackInfoWindow] = []
    var logWindows: [NSWindowController] = []
    var listsSplit: NSSplitView?
    var keyMonitor: Any?
    var wheelMonitor: Any?
    var highPriorityToken: NSObjectProtocol?
    var statusItem: NSStatusItem?
    var findWindow: FindFilesWindow?
    weak var searchField: NSSearchField?
    var coverContainer: NSView?
    var coverImage: NSImageView?
    var autosaveTimer: Timer?
    var tickCount = 0
    var remote: RemoteServer?
    var bigTitle: BigTitleWindow?
    var settingsWindow: SettingsWindow?

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
        win.minSize = NSSize(width: 560, height: 420)
        super.init(window: win)
        win.delegate = self
        buildContent(in: win)
        buildToolbar(for: win)
        wirePlayer()
        installKeyMonitor()
        player.onQuitRequested = { NSApp.terminate(nil) }
        if player.settings.httpEnabled { toggleRemote() }
        applySettings()
        applyCompact()
        ticker = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            self?.tick()
        }
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    // MARK: - Layout

    private func buildContent(in win: NSWindow) {
        tree.onSelect = { [weak self] url in self?.openFolder(url) }
        table.onActivate = { [weak self] row in
            guard let self, self.player.tracks.indices.contains(row) else { return }
            let t = self.player.tracks[row]
            if t.isFolder { self.openFolder(t.url); self.tree.select(t.url) } else { self.player.playFolder(row: row) }
        }
        table.onSelectionChange = { [weak self] in self?.updateStatus() }
        table.onSort = { [weak self] key, desc in
            self?.player.sortFolder(by: key, descending: desc)
            self?.table.setTracks(self?.player.folderTracks ?? [])
        }

        let treeVC = NSViewController()
        treeVC.view = makeSidebar()
        let contentVC = NSViewController()
        contentVC.view = makeContentView()

        let split = NSSplitViewController()
        let sidebar = NSSplitViewItem(sidebarWithViewController: treeVC)
        sidebar.minimumThickness = 160
        sidebar.maximumThickness = 420
        sidebar.canCollapse = true
        sidebar.isCollapsed = !player.settings.showTree
        let content = NSSplitViewItem(viewController: contentVC)
        content.minimumThickness = 320
        split.addSplitViewItem(sidebar)
        split.addSplitViewItem(content)
        splitController = split
        win.contentViewController = split
        // Assigning the controller resizes the window to its fitting size. Restore the default.
        win.setContentSize(NSSize(width: 1100, height: 680))
        win.center()
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
        // A plain container pins the label to the leading edge. A stack view would centre it.
        let statusBox = NSView()
        statusBox.addSubview(statusLabel)
        NSLayoutConstraint.activate([
            statusLabel.leadingAnchor.constraint(equalTo: statusBox.leadingAnchor, constant: 12),
            statusLabel.trailingAnchor.constraint(lessThanOrEqualTo: statusBox.trailingAnchor, constant: -12),
            statusLabel.topAnchor.constraint(equalTo: statusBox.topAnchor, constant: 3),
            statusLabel.bottomAnchor.constraint(equalTo: statusBox.bottomAnchor, constant: -4),
        ])

        let separator = NSBox()
        separator.boxType = .separator
        separator.translatesAutoresizingMaskIntoConstraints = false

        playlistTable.onActivate = { [weak self] row in self?.player.playFromPlaylist(row: row) }
        let lists = NSSplitView()
        lists.dividerStyle = .thin
        listsSplit = lists
        rebuildLists()
        lists.translatesAutoresizingMaskIntoConstraints = false
        // The lists take all the height the transport and status rows leave. Without this the
        // split collapses to zero height and the track list is invisible.
        lists.setContentHuggingPriority(NSLayoutConstraint.Priority(1), for: .vertical)
        lists.setContentCompressionResistancePriority(.defaultLow, for: .vertical)
        lists.heightAnchor.constraint(greaterThanOrEqualToConstant: 120).isActive = true
        transport.setContentHuggingPriority(.required, for: .vertical)
        statusLabel.setContentHuggingPriority(.required, for: .vertical)
        // A vertical stack: seek and volume on top, the lists filling the middle, status at the bottom.
        let column = NSStackView(views: [transport, separator, lists, statusBox])
        column.orientation = .vertical
        column.alignment = .leading
        column.spacing = 0
        column.distribution = .fill
        column.translatesAutoresizingMaskIntoConstraints = false
        root.addSubview(column)
        NSLayoutConstraint.activate([
            column.topAnchor.constraint(equalTo: root.topAnchor),
            column.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            column.leadingAnchor.constraint(equalTo: root.leadingAnchor),
            column.trailingAnchor.constraint(equalTo: root.trailingAnchor),
            transport.widthAnchor.constraint(equalTo: column.widthAnchor),
            separator.widthAnchor.constraint(equalTo: column.widthAnchor),
            lists.widthAnchor.constraint(equalTo: column.widthAnchor),
            statusBox.widthAnchor.constraint(equalTo: column.widthAnchor),
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
            searchField = field
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
        player.onTrackHeard = { [weak self] row in
            guard let self, !self.player.usesPlaylist else { return }
            let focusedHere = self.window?.firstResponder === self.table.table
            guard self.player.settings.goHomeOnChange || (self.player.settings.continueFocused && focusedHere) else { return }
            self.table.select(row: row)
            self.table.scrollToRow(row)
        }
        player.onFolderEnd = { [weak self] in self?.folderEnded() }
        tree.setRoots(initialRoots())
        let last = player.settings.lastFolder.map(URL.init(fileURLWithPath:))
        if let last, player.settings.restoreList, !player.settings.startInBase, FileManager.default.fileExists(atPath: last.path) {
            // A folder outside the tree roots becomes a root, so the tree can show it.
            if !tree.roots.contains(where: { last.path.hasPrefix($0.path) }) { tree.add(root: last) }
            openFolder(last)
            tree.select(last)
        } else if let first = tree.roots.first {
            openFolder(first)
            tree.select(first)
        }
    }

    private func initialRoots() -> [URL] {
        if !player.settings.roots.isEmpty {
            return player.settings.roots.map { URL(fileURLWithPath: $0) }
        }
        // The home folder is the tree root, so its subfolders show as the tree.
        return [FileManager.default.homeDirectoryForCurrentUser]
    }

    /// Load a folder: list it at once, then fill in tags on a background queue.
    func openFolder(_ requested: URL) {
        var url = requested
        var rows = folderRows(url, withFolders: player.settings.foldersInList)
        if rows.isEmpty, player.settings.autoSearchDirs, let found = firstFolderWithAudio(requested, depth: 4) {
            url = found
            rows = folderRows(found, withFolders: player.settings.foldersInList)
        }
        player.setFolder(url, tracks: rows)
        recordHistory(url)
        table.setTracks(rows)
        if let idx = player.resumeRow {
            table.select(row: idx)
            table.scrollToRow(idx)
        }
        let always = player.settings.alwaysScan
        DispatchQueue.global(qos: .userInitiated).async {
            for t in rows where t.info == nil { t.info = cachedInfo(t, reuse: !always) }
            DispatchQueue.main.async { [weak self] in
                guard let self, self.player.folder == url else { return }
                self.table.reload()
                self.refresh()
            }
        }
        window?.title = "1by1 - \(url.lastPathComponent)"
        updateCover()
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
        player.settings.showTree = item.isCollapsed == false
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
            window?.title = windowTitle(for: t)
        }
        bigTitle?.update(track: player.heardTrack, position: player.position, duration: player.duration,
                         layout: player.settings.bigLayout, pattern: player.settings.bigInfoPattern,
                         padding: player.settings.bigPadding, cycleSeconds: player.settings.bigCycleSecs)
        if player.isPaused { statusLabel.stringValue = "Paused" }
        updateStatus()
        tick()
    }

    private func tick() {
        periodicWork()
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
        let matches: (Track) -> Bool = { containsCI($0.display, text) || containsCI($0.name, text) }
        if let row = player.folderTracks.firstIndex(where: matches) {
            table.select(row: row)
            return
        }
        // Skip Lists in Search off: a miss in the folder list continues into the playlist.
        guard !player.settings.skipListsInSearch, let row = player.playlist.firstIndex(where: matches) else { return }
        playlistTable.select(row: row)
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
        let url = URL(fileURLWithPath: player.settings.recentDirs[sender.tag])
        openFolder(url)
        tree.select(url)
    }

    /// Called from openFolder: remember the folder at the top of recentDirs.
    func recordHistory(_ folder: URL) {
        var s = player.settings
        s.recentDirs.removeAll { $0 == folder.path }
        s.recentDirs.insert(folder.path, at: 0)
        s.recentDirs = Array(s.recentDirs.prefix(BookmarksMenu.maxHistory))
        player.settings = s
        player.save()
    }
}

// MARK: - HTTP remote

extension MainWindowController {
    /// Run a remote command. Paths match the Rust build.
    func runRemote(_ path: String) {
        switch path {
        case "/res": player.togglePause(selectedRow: table.selectedRow)
        case "/prv": player.prev()
        case "/nxt": player.next()
        case "/rew": player.rewind(by: -5)
        case "/fwd": player.rewind(by: 5)
        case "/vdn": player.setVolume(player.settings.volume - 0.05)
        case "/vup": player.setVolume(player.settings.volume + 0.05)
        case "/dprv": stepFolder(-1)
        case "/dnxt": stepFolder(1)
        case "/home": if let row = player.heard { table.select(row: row) }
        default: break
        }
    }

    /// Open the sibling folder before or after the current one.
    func stepFolder(_ delta: Int) {
        guard let folder = player.folder else { return }
        let siblings = subfolders(folder.deletingLastPathComponent())
        guard let i = siblings.firstIndex(of: folder) else { return }
        let j = i + delta
        guard siblings.indices.contains(j) else { return }
        openFolder(siblings[j])
        tree.select(siblings[j])
    }

    /// Start or stop the remote. Port and state persist in settings.
    @objc func toggleRemote() {
        if let remote {
            remote.stop()
            self.remote = nil
            player.settings.httpEnabled = false
        } else {
            let r = RemoteServer(port: remotePort) { [weak self] path in self?.runRemote(path) }
            do {
                try r.start()
                remote = r
                player.settings.httpEnabled = true
            } catch {
                statusLabel.stringValue = "Remote failed on port \(r.port): \(error.localizedDescription)"
            }
        }
        player.save()
    }
}

extension MainWindowController {
    @objc func toggleBigTitle() {
        if let bigTitle, bigTitle.window?.isVisible == true {
            bigTitle.window?.close()
            return
        }
        let w = bigTitle ?? BigTitleWindow()
        bigTitle = w
        w.showWindow(nil)
        refresh()
    }

    @objc func showSettings() {
        let w = settingsWindow ?? SettingsWindow(player: player)
        w.onRemoteToggle = { [weak self] on in self?.setRemote(on) }
        w.onApply = { [weak self] in self?.applySettings() }
        settingsWindow = w
        w.showWindow(nil)
    }

    /// Explicit start or stop of the remote, for the settings checkbox.
    func setRemote(_ on: Bool) {
        if on, remote == nil { toggleRemote() }
        if !on, remote != nil { toggleRemote() }
    }
}

extension MainWindowController {
    /// Folder reached its end with repeat off: reload it or move to the next folder.
    func folderEnded() {
        let s = player.settings
        if s.reloadAtEnd, let folder = player.folder {
            openFolder(folder)
            if !player.tracks.isEmpty { player.playFolder(row: 0) }
        } else if s.switchToFolderAtEnd {
            stepFolder(1)
            if !player.tracks.isEmpty { player.playFolder(row: 0) }
        }
    }
}
