import AppKit

/// Basic preferences: remote, startup folder tree, default repeat and shuffle. The rest of the Rust
/// build's eleven pages are not ported.
final class SettingsWindow: NSWindowController {
    private let player: PlayerController
    private let remoteBox = NSButton(checkboxWithTitle: "Enable HTTP remote on 127.0.0.1:8080", target: nil, action: nil)
    private let treeBox = NSButton(checkboxWithTitle: "Show folder tree", target: nil, action: nil)
    private let shuffleBox = NSButton(checkboxWithTitle: "Shuffle", target: nil, action: nil)
    private let repeatPop = NSPopUpButton(frame: .zero, pullsDown: false)
    var onRemoteToggle: ((Bool) -> Void)?

    init(player: PlayerController) {
        self.player = player
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 420, height: 230),
                           styleMask: [.titled, .closable], backing: .buffered, defer: false)
        win.title = "Settings"
        super.init(window: win)
        repeatPop.addItems(withTitles: ["Repeat off", "Repeat all", "Repeat track"])
        remoteBox.target = self
        remoteBox.action = #selector(remoteChanged)
        treeBox.target = self
        treeBox.action = #selector(treeChanged)
        shuffleBox.target = self
        shuffleBox.action = #selector(shuffleChanged)
        repeatPop.target = self
        repeatPop.action = #selector(repeatChanged)
        load()

        let stack = NSStackView(views: [remoteBox, treeBox, shuffleBox, repeatPop])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 20, bottom: 20, right: 20)
        win.contentView = stack
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    private func load() {
        let s = player.settings
        remoteBox.state = s.httpEnabled ? .on : .off
        treeBox.state = s.treeVisible ? .on : .off
        shuffleBox.state = player.shuffle ? .on : .off
        repeatPop.selectItem(at: [Repeat.off, .all, .one].firstIndex(of: player.repeatMode) ?? 0)
    }

    @objc private func remoteChanged() { onRemoteToggle?(remoteBox.state == .on) }
    @objc private func treeChanged() {
        player.settings.treeVisible = treeBox.state == .on
        player.save()
    }
    @objc private func shuffleChanged() { player.shuffle = shuffleBox.state == .on }
    @objc private func repeatChanged() {
        player.repeatMode = [Repeat.off, .all, .one][repeatPop.indexOfSelectedItem]
    }
}
