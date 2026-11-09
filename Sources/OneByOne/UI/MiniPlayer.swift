import AppKit

/// Menu bar dropdown in the style of VLC: artwork, title, transport buttons and time.
/// Lives in the status item's menu. Refreshes every half second while the menu is open.
final class MiniPlayer: NSObject, NSMenuDelegate {
    let view = NSView(frame: NSRect(x: 0, y: 0, width: 280, height: 140))
    private weak var controller: MainWindowController?
    private let player: PlayerController
    private let artwork = NSImageView(frame: NSRect(x: 12, y: 64, width: 64, height: 64))
    private let titleLabel = NSTextField(labelWithString: "")
    private let artistLabel = NSTextField(labelWithString: "")
    private let timeLabel = NSTextField(labelWithString: "")
    private let playPause = NSButton()
    private let shuffle = NSButton()
    private var timer: Timer?
    private var artFolder: URL?
    private var cachedArt: NSImage?

    init(controller: MainWindowController, player: PlayerController) {
        self.controller = controller
        self.player = player
        super.init()

        titleLabel.frame = NSRect(x: 88, y: 110, width: 180, height: 18)
        titleLabel.font = .systemFont(ofSize: 13, weight: .semibold)
        titleLabel.lineBreakMode = .byTruncatingTail
        artistLabel.frame = NSRect(x: 88, y: 92, width: 180, height: 16)
        artistLabel.font = .systemFont(ofSize: 11)
        artistLabel.textColor = .secondaryLabelColor
        artistLabel.lineBreakMode = .byTruncatingTail
        timeLabel.frame = NSRect(x: 12, y: 6, width: 256, height: 16)
        timeLabel.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
        timeLabel.textColor = .secondaryLabelColor
        timeLabel.alignment = .center
        artwork.imageScaling = .scaleProportionallyUpOrDown
        artwork.wantsLayer = true
        artwork.layer?.cornerRadius = 6
        artwork.layer?.masksToBounds = true

        for v in [artwork, titleLabel, artistLabel, timeLabel] { view.addSubview(v) }
        button(NSButton(), "backward.end.fill", x: 68, action: #selector(MainWindowController.previousTrack))
        button(playPause, "play.fill", x: 104, action: #selector(MainWindowController.playPausePressed))
        button(NSButton(), "forward.end.fill", x: 140, action: #selector(MainWindowController.nextTrack))
        button(shuffle, "shuffle", x: 176, action: #selector(MainWindowController.toggleShuffle))
        update()
    }

    private func button(_ b: NSButton, _ symbol: String, x: CGFloat, action: Selector) {
        b.frame = NSRect(x: x, y: 26, width: 32, height: 28)
        b.isBordered = false
        b.imagePosition = .imageOnly
        b.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        b.target = controller
        b.action = action
        view.addSubview(b)
    }

    /// Copy the player state into the labels, buttons and artwork.
    func update() {
        let track = player.heardTrack
        titleLabel.stringValue = track?.display ?? "1by1"
        artistLabel.stringValue = track == nil ? "Nothing playing" : (track?.info?.tags.artist ?? "")
        timeLabel.stringValue = track == nil
            ? "--:-- / --:--"
            : "\(clock(player.position)) / \(player.duration.map { clock($0) } ?? "--:--")"
        playPause.image = NSImage(systemSymbolName: player.isPlaying ? "pause.fill" : "play.fill",
                                  accessibilityDescription: nil)
        shuffle.contentTintColor = player.shuffle ? .controlAccentColor : .secondaryLabelColor
        setArtwork(track?.url.deletingLastPathComponent())
    }

    /// Artwork comes from the playing track's folder. Cached per folder, so the tick does not reread it.
    private func setArtwork(_ folder: URL?) {
        guard let folder else {
            artFolder = nil
            cachedArt = nil
            artwork.image = placeholder
            return
        }
        if folder != artFolder {
            artFolder = folder
            cachedArt = coverArt(in: folder, names: player.settings.coverNames)
        }
        artwork.image = cachedArt ?? placeholder
    }

    private var placeholder: NSImage? { NSImage(systemSymbolName: "music.note", accessibilityDescription: nil) }

    func menuWillOpen(_ menu: NSMenu) {
        update()
        // Common modes: the default-mode timers do not fire while a menu tracks the mouse.
        let t = Timer(timeInterval: 0.5, repeats: true) { [weak self] _ in self?.update() }
        RunLoop.main.add(t, forMode: .common)
        timer = t
    }

    func menuDidClose(_ menu: NSMenu) { stop() }

    func stop() {
        timer?.invalidate()
        timer = nil
    }
}
