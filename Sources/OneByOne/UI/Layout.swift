import AppKit

/// Settings-driven layout: sidebar cover panel, playlist position, list style and the autosave timer.
extension MainWindowController {
    /// Folder tree above the cover panel.
    func makeSidebar() -> NSView {
        let box = NSStackView()
        box.orientation = .vertical
        box.spacing = 0
        box.alignment = .leading
        tree.view.setContentHuggingPriority(.defaultLow, for: .vertical)
        let cover = NSView()
        cover.translatesAutoresizingMaskIntoConstraints = false
        let image = NSImageView()
        image.imageScaling = .scaleProportionallyUpOrDown
        image.translatesAutoresizingMaskIntoConstraints = false
        cover.addSubview(image)
        NSLayoutConstraint.activate([
            image.leadingAnchor.constraint(equalTo: cover.leadingAnchor, constant: 8),
            image.trailingAnchor.constraint(equalTo: cover.trailingAnchor, constant: -8),
            image.topAnchor.constraint(equalTo: cover.topAnchor, constant: 8),
            image.bottomAnchor.constraint(equalTo: cover.bottomAnchor, constant: -8),
            cover.heightAnchor.constraint(equalToConstant: CGFloat(player.settings.coverHeight)),
        ])
        coverContainer = cover
        coverImage = image
        box.addArrangedSubview(tree.view)
        box.addArrangedSubview(cover)
        tree.view.widthAnchor.constraint(equalTo: box.widthAnchor).isActive = true
        cover.widthAnchor.constraint(equalTo: box.widthAnchor).isActive = true
        return box
    }

    /// Put the playlist top, bottom or right of the track list, shown or hidden.
    func rebuildLists() {
        guard let lists = listsSplit else { return }
        let s = player.settings
        lists.arrangedSubviews.forEach { $0.removeFromSuperview() }
        lists.isVertical = s.playlistPos == .right
        if s.playlistOpen {
            if s.playlistPos == .top {
                lists.addArrangedSubview(playlistTable.view)
                lists.addArrangedSubview(table.view)
            } else {
                lists.addArrangedSubview(table.view)
                lists.addArrangedSubview(playlistTable.view)
            }
        } else {
            lists.addArrangedSubview(table.view)
        }
        lists.setHoldingPriority(NSLayoutConstraint.Priority(250), forSubviewAt: 0)
    }

    /// Show the cover file from the current folder, or hide the panel when there is none.
    func updateCover() {
        let s = player.settings
        guard let container = coverContainer, let image = coverImage else { return }
        container.isHidden = !s.coverArt || s.coverMode == .off
        guard !container.isHidden, let folder = player.folder else { return }
        let names = s.coverNames.split(separator: ";").map { $0.trimmingCharacters(in: .whitespaces).lowercased() }
        let files = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? []
        let hit = files.first { names.contains($0.lastPathComponent.lowercased()) }
        if let hit, let img = NSImage(contentsOf: hit) {
            image.image = img
            container.isHidden = false
        } else {
            image.image = nil
            container.isHidden = s.coverHideEmpty
        }
    }

    /// Re-apply every setting that the window shows.
    func applySettings() {
        let s = player.settings
        displayUsesTags = s.tagTitles
        infoModeSetting = s.infoMode
        clockColonMinutes = s.colonMinutes
        for t in [table, playlistTable] {
            t.sizeWithoutLength = s.noLengthInSize
            t.applyStyle(grid: s.listGrid, hideScrollbars: s.hideScrollbars,
                         rowHeight: s.biggerTree ? 22 : 18, showInfo: s.extraInfoCol)
        }
        if let item = splitController?.splitViewItems.first, item.isCollapsed == s.showTree {
            item.isCollapsed = !s.showTree
        }
        volumeSlider.controlSize = s.largerSliders ? .regular : .small
        for item in window?.toolbar?.items ?? [] { item.toolTip = s.showTooltips ? item.label : nil }
        tree.applyStyle(rowSpacing: s.treeSpacing, bigger: s.biggerTree, lines: s.treeLines)
        rebuildLists()
        updateCover()
        setHistoryMenuHidden(!s.historyMenus)
        if s.bigView, bigTitle?.window?.isVisible != true { toggleBigTitle() }
        if !s.bigView, bigTitle?.window?.isVisible == true { bigTitle?.window?.close() }
        startAutosave()
        applyExtras()
        table.reload()
        playlistTable.reload()
        refresh()
    }

    /// Port from `httpAddr`, such as "127.0.0.1:8080". Falls back to 8080.
    var remotePort: UInt16 {
        let text = player.settings.httpAddr.split(separator: ":").last.map(String.init) ?? ""
        return UInt16(text) ?? RemoteServer.defaultPort
    }

    private func setHistoryMenuHidden(_ hidden: Bool) {
        let file = NSApp.mainMenu?.items.first { $0.submenu?.title == "File" }?.submenu
        file?.items.first { $0.title == "History" }?.isHidden = hidden
    }

    private func startAutosave() {
        autosaveTimer?.invalidate()
        autosaveTimer = nil
        guard player.settings.autosave5min else { return }
        autosaveTimer = Timer.scheduledTimer(withTimeInterval: 300, repeats: true) { [weak self] _ in
            self?.player.rememberResume()
            self?.player.save()
        }
    }
}
