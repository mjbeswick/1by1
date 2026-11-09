import AppKit
import ObjectiveC

/// A value with a display label, for popup menus.
protocol Labeled { var label: String { get } }
extension Repeat: Labeled {}
extension PlaylistPos: Labeled {}
extension CoverMode: Labeled {}
extension BigLayout: Labeled {}
extension TitleMode: Labeled {}
extension InfoMode: Labeled {}
extension SortKey: Labeled {}
extension Theme: Labeled {}

/// Preferences in eleven pages. Every control writes its field and calls `onApply`.
final class SettingsWindow: NSWindowController, NSTableViewDataSource, NSTableViewDelegate {
    private let player: PlayerController
    var onApply: (() -> Void)?
    var onRemoteToggle: ((Bool) -> Void)?

    private let pageNames = ["Playback", "Lists and info", "Controls", "Window style", "Titles",
                             "Panels", "Big view", "System", "Logging and history", "External", "Hotkeys", "About"]
    private let pageList = NSTableView()
    private let pageHost = NSView()
    private var pages: [NSView] = []

    init(player: PlayerController) {
        self.player = player
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 660, height: 520),
                           styleMask: [.titled, .closable], backing: .buffered, defer: false)
        win.title = "Settings"
        super.init(window: win)
        buildPages()
        buildLayout(win)
        pageList.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
        showPage(0)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    private func buildLayout(_ win: NSWindow) {
        let col = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("page"))
        col.width = 160
        pageList.addTableColumn(col)
        pageList.headerView = nil
        pageList.style = .sourceList
        pageList.dataSource = self
        pageList.delegate = self
        let listScroll = NSScrollView()
        listScroll.documentView = pageList
        listScroll.hasVerticalScroller = true
        listScroll.widthAnchor.constraint(equalToConstant: 170).isActive = true

        pageHost.translatesAutoresizingMaskIntoConstraints = false
        let root = NSStackView(views: [listScroll, pageHost])
        root.orientation = .horizontal
        root.alignment = .top
        root.edgeInsets = NSEdgeInsets(top: 12, left: 12, bottom: 12, right: 12)
        root.translatesAutoresizingMaskIntoConstraints = false
        win.contentView?.addSubview(root)
        NSLayoutConstraint.activate([
            root.leadingAnchor.constraint(equalTo: win.contentView!.leadingAnchor),
            root.trailingAnchor.constraint(equalTo: win.contentView!.trailingAnchor),
            root.topAnchor.constraint(equalTo: win.contentView!.topAnchor),
            root.bottomAnchor.constraint(equalTo: win.contentView!.bottomAnchor),
        ])
    }

    private func showPage(_ i: Int) {
        pageHost.subviews.forEach { $0.removeFromSuperview() }
        let page = pages[i]
        page.translatesAutoresizingMaskIntoConstraints = false
        pageHost.addSubview(page)
        NSLayoutConstraint.activate([
            page.leadingAnchor.constraint(equalTo: pageHost.leadingAnchor),
            page.trailingAnchor.constraint(lessThanOrEqualTo: pageHost.trailingAnchor),
            page.topAnchor.constraint(equalTo: pageHost.topAnchor),
            page.bottomAnchor.constraint(lessThanOrEqualTo: pageHost.bottomAnchor),
        ])
    }

    func numberOfRows(in tableView: NSTableView) -> Int { pageNames.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let cell = NSTextField(labelWithString: pageNames[row])
        cell.font = .systemFont(ofSize: 13)
        return cell
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        let r = pageList.selectedRow
        if r >= 0 { showPage(r) }
    }

    // MARK: - Control helpers

    private func apply() {
        player.save()
        onApply?()
    }

    private func check(_ title: String, _ kp: WritableKeyPath<Settings, Bool>) -> NSButton {
        let b = NSButton(checkboxWithTitle: title, target: nil, action: nil)
        b.state = player.settings[keyPath: kp] ? .on : .off
        b.onToggle { [weak self] on in
            guard let self else { return }
            self.player.settings[keyPath: kp] = on
            self.apply()
        }
        return b
    }

    private func number(_ title: String, _ kp: WritableKeyPath<Settings, Int>, _ range: ClosedRange<Int>) -> NSView {
        let field = NSTextField(string: "\(player.settings[keyPath: kp])")
        field.alignment = .right
        field.widthAnchor.constraint(equalToConstant: 70).isActive = true
        let stepper = NSStepper()
        stepper.minValue = Double(range.lowerBound)
        stepper.maxValue = Double(range.upperBound)
        stepper.integerValue = player.settings[keyPath: kp]
        stepper.onChange { [weak self] v in
            guard let self else { return }
            self.player.settings[keyPath: kp] = v
            field.stringValue = "\(v)"
            self.apply()
        }
        field.onCommit { [weak self] text in
            guard let self, let v = Int(text), range.contains(v) else { field.stringValue = "\(self?.player.settings[keyPath: kp] ?? 0)"; return }
            self.player.settings[keyPath: kp] = v
            stepper.integerValue = v
            self.apply()
        }
        return row(title, [field, stepper])
    }

    private func choice<T: RawRepresentable & CaseIterable & Labeled & Equatable>(_ title: String, _ kp: WritableKeyPath<Settings, T>) -> NSView
    where T.RawValue == String {
        let popup = NSPopUpButton(frame: .zero, pullsDown: false)
        let all = Array(T.allCases)
        popup.addItems(withTitles: all.map { $0.label })
        popup.selectItem(at: all.firstIndex(of: player.settings[keyPath: kp]) ?? 0)
        popup.onSelect { [weak self] index in
            guard let self else { return }
            self.player.settings[keyPath: kp] = all[index]
            self.apply()
        }
        return row(title, [popup])
    }

    private func text(_ title: String, _ kp: WritableKeyPath<Settings, String>, width: CGFloat = 260) -> NSView {
        let field = NSTextField(string: player.settings[keyPath: kp])
        field.widthAnchor.constraint(equalToConstant: width).isActive = true
        field.onCommit { [weak self] v in
            guard let self else { return }
            self.player.settings[keyPath: kp] = v
            self.apply()
        }
        return row(title, [field])
    }

    private func row(_ title: String, _ views: [NSView]) -> NSView {
        let label = NSTextField(labelWithString: title)
        label.alignment = .right
        label.widthAnchor.constraint(equalToConstant: 200).isActive = true
        return NSStackView(views: [label] + views).then { $0.orientation = .horizontal; $0.spacing = 8 }
    }

    private func page(_ items: [NSView]) -> NSView {
        let stack = NSStackView(views: items)
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.edgeInsets = NSEdgeInsets(top: 8, left: 16, bottom: 8, right: 16)
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.drawsBackground = false
        let doc = NSView()
        doc.translatesAutoresizingMaskIntoConstraints = false
        stack.translatesAutoresizingMaskIntoConstraints = false
        doc.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: doc.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: doc.trailingAnchor),
            stack.topAnchor.constraint(equalTo: doc.topAnchor),
            stack.bottomAnchor.constraint(equalTo: doc.bottomAnchor),
        ])
        scroll.documentView = doc
        doc.widthAnchor.constraint(equalTo: scroll.contentView.widthAnchor).isActive = true
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.widthAnchor.constraint(equalToConstant: 460).isActive = true
        scroll.heightAnchor.constraint(equalToConstant: 470).isActive = true
        return scroll
    }

    // MARK: - Pages

    private func buildPages() {
        pages = [
            page([
                check("Gapless playback", \.gapless),
                check("Play one track, then stop", \.oneTrack),
                check("Remember positions per folder", \.rememberPositions),
                number("Resume back (ms)", \.resumeBackMs, 0...5000),
                number("Rewind seconds", \.rewindSecs, 1...300),
                number("Forward seconds", \.forwardSecs, 1...300),
                check("Next track at end of forward", \.nextAtEndOfFFwd),
                number("Auto space (ms)", \.autoSpaceMs, 0...10000),
                number("Fade on stop (ms)", \.fadeStopMs, 0...10000),
                number("Fade on skip (ms)", \.fadeSkipMs, 0...10000),
                check("Go to current track on change", \.goHomeOnChange),
                check("Clear selection on change", \.unselectOnChange),
                check("Remove played tracks", \.removePlayed),
                check("Switch to next folder at end", \.switchToFolderAtEnd),
                check("Reload folder at end", \.reloadAtEnd),
                check("Start in base folder", \.startInBase),
                check("Continue in focused list", \.continueFocused),
                choice("Repeat", \.repeatMode),
            ]),
            page([
                check("Restore list on start", \.restoreList),
                check("Show folders in list", \.foldersInList),
                check("Natural sort", \.naturalSort),
                check("Lazy folder tree", \.lazyTree),
                check("Extra info column", \.extraInfoCol),
                choice("Info column", \.infoMode),
                choice("Rating style", \.ratingStyle),
                check("Hide length in size column", \.noLengthInSize),
                check("Always scan tags", \.alwaysScan),
                check("Hide scrollbars", \.hideScrollbars),
                check("Skip lists in search", \.skipListsInSearch),
                check("Auto search directories", \.autoSearchDirs),
            ]),
            page([
                check("Mouse wheel changes volume", \.wheelVolume),
                check("Mouse wheel scrolls tree", \.wheelTree),
                check("Mouse wheel scrolls lists", \.wheelLists),
                check("Fine volume steps", \.fineVolume),
                check("Larger sliders", \.largerSliders),
                check("Show tooltips", \.showTooltips),
                check("Tab moves between toolbar and lists", \.tabToolbar),
                check("Escape quits", \.escQuit),
            ]),
            page([
                choice("Theme", \.theme),
                check("Grid lines in list", \.listGrid),
                check("Full-row selection", \.fullRow),
                check("Tree lines", \.treeLines),
                check("Tree spacing", \.treeSpacing),
                check("Explorer-style tree", \.treeStyleExplorer),
                number("Dim inactive list (%)", \.dimInactiveList, 0...100),
                check("Bigger tree rows", \.biggerTree),
                check("Show folder tree", \.showTree),
            ]),
            page([
                choice("Title", \.titleMode),
                check("Use tag titles", \.tagTitles),
                check("Show upcoming track in title", \.upcomingInTitle),
                check("Minutes with colon (m:ss)", \.colonMinutes),
            ]),
            page([
                check("Cover art panel", \.coverArt),
                choice("Panel shows", \.coverMode),
                check("Hide panel when no art", \.coverHideEmpty),
                text("Cover file names", \.coverNames, width: 220),
                number("Cover height", \.coverHeight, 80...600),
                number("Tree width", \.treeWidth, 140...600),
                choice("Playlist position", \.playlistPos),
                check("Playlist open", \.playlistOpen),
                number("Playlist height", \.playlistHeight, 80...800),
                number("Playlist width", \.playlistWidth, 160...800),
            ]),
            page([
                check("Big view", \.bigView),
                choice("Layout", \.bigLayout),
                number("Idle seconds before hiding", \.bigIdleSecs, 0...3600),
                number("Padding", \.bigPadding, 0...80),
                number("Info cycle seconds", \.bigCycleSecs, 1...600),
                text("Info pattern", \.bigInfoPattern, width: 260),
            ]),
            page([
                check("Only one instance", \.oneInstance),
                check("Status bar icon", \.trayIcon),
                check("High priority", \.highPriority),
                check("Minimize on close", \.minimizeOnClose),
                check("Save every 5 minutes", \.autosave5min),
                remoteControl(),
                text("Remote address", \.httpAddr, width: 160),
            ]),
            page([
                check("Track log", \.logEnabled),
                number("Log size limit (KB, 0 = none)", \.logMaxKB, 0...100000),
                check("History menus", \.historyMenus),
                number("History entries", \.historyEntries, 1...50),
            ]),
            page([
                text("External 1", \.external[0], width: 300),
                text("External 2", \.external[1], width: 300),
                text("External 3", \.external[2], width: 300),
                NSTextField(wrappingLabelWithString: "Use %1 for the file path. Example: open -a Sublime\\ Text %1"),
            ]),
            page(hotkeyRows()),
            page([
                NSTextField(wrappingLabelWithString: "1by1 for macOS. A folder is the queue. Settings in ~/Library/Application Support/1by1/settings.json. Track log in 1by1.log beside it."),
            ]),
        ]
    }

    private func remoteControl() -> NSView {
        check("HTTP remote on 127.0.0.1", \.httpEnabled).then {
            $0.target = self
            $0.action = #selector(remoteToggled(_:))
        }
    }

    @objc private func remoteToggled(_ sender: NSButton) {
        player.settings.httpEnabled = sender.state == .on
        apply()
        onRemoteToggle?(sender.state == .on)
    }

    private func hotkeyRows() -> [NSView] {
        let keys: [(String, String)] = [
            ("Space", "Play / pause"), ("Return", "Play selected"), ("Left / Right", "Rewind / forward"),
            ("Ctrl-Left / Ctrl-Right", "Previous / next track"), ("Shift-Left / Shift-Right", "Previous / next folder"),
            ("Ctrl-Return", "Add to playlist"), ("Ctrl-R", "Repeat"), ("Ctrl-P", "Shuffle"),
            ("Ctrl-N", "Intro scan"), ("Ctrl-X", "Explore folder"), ("Ctrl-B", "Big title view"),
            ("F4", "Folder tree"), ("Cmd-G", "Go to track"), ("Cmd-I", "Track info"),
            ("Cmd-D", "Add bookmark"), ("Cmd-Shift-L", "Track log"), ("Cmd-,", "Settings"),
        ]
        return keys.map { row($0.0, [NSTextField(labelWithString: $0.1)]) }
    }
}

extension NSButton {
    /// Runs `handler` with the new on/off state when the box changes.
    func onToggle(_ handler: @escaping (Bool) -> Void) {
        target = ClosureTarget.make { handler(self.state == .on) }
        action = #selector(ClosureTarget.fire(_:))
    }
}

extension NSStepper {
    func onChange(_ handler: @escaping (Int) -> Void) {
        target = ClosureTarget.make { handler(self.integerValue) }
        action = #selector(ClosureTarget.fire(_:))
    }
}

extension NSTextField {
    /// Runs `handler` with the text when editing ends.
    func onCommit(_ handler: @escaping (String) -> Void) {
        target = ClosureTarget.make { handler(self.stringValue) }
        action = #selector(ClosureTarget.fire(_:))
    }
}

extension NSPopUpButton {
    func onSelect(_ handler: @escaping (Int) -> Void) {
        target = ClosureTarget.make { handler(self.indexOfSelectedItem) }
        action = #selector(ClosureTarget.fire(_:))
    }
}

/// Holds a closure as an Objective-C target. Kept alive by the control it is attached to.
final class ClosureTarget: NSObject {
    private let body: () -> Void
    private init(_ body: @escaping () -> Void) { self.body = body }

    /// The target retains itself, so it lives as long as the control that calls it.
    static func make(_ body: @escaping () -> Void) -> ClosureTarget {
        let t = ClosureTarget(body)
        objc_setAssociatedObject(t, closureTargetKey, t, .OBJC_ASSOCIATION_RETAIN)
        return t
    }

    @objc func fire(_ sender: Any?) { body() }
}

extension NSStackView {
    func then(_ f: (NSStackView) -> Void) -> NSStackView { f(self); return self }
}

extension NSButton {
    func then(_ f: (NSButton) -> Void) -> NSButton { f(self); return self }
}

private var closureTargetKey: UnsafeRawPointer { UnsafeRawPointer(Unmanaged.passUnretained(ClosureTargetKeyBox.shared).toOpaque()) }
private final class ClosureTargetKeyBox { static let shared = ClosureTargetKeyBox() }
