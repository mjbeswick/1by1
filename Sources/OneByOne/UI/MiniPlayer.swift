import AppKit

/// Menu bar dropdown: artwork, title and details, seek, transport, volume. The status item menu
/// holds Show Main Window and Quit below it. Refreshes every half second while the menu is open.
final class MiniPlayer: NSObject, NSMenuDelegate {
    static let width: CGFloat = 340
    static let height: CGFloat = 220
    /// Centre line of the transport row, measured from the top.
    private static let transportTop: CGFloat = 146
    /// Centre line of the volume row, measured from the top.
    private static let volumeTop: CGFloat = 200

    let view = NSView(frame: NSRect(x: 0, y: 0, width: MiniPlayer.width, height: MiniPlayer.height))
    private weak var controller: MainWindowController?
    private let player: PlayerController
    private let artwork = NSImageView()
    private let titleLabel = NSTextField(labelWithString: "")
    private let detailLabel = NSTextField(labelWithString: "")
    private let elapsedLabel = NSTextField(labelWithString: "")
    private let totalLabel = NSTextField(labelWithString: "")
    private let seek = NSSlider(value: 0, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let volume = NSSlider(value: 0, minValue: 0, maxValue: 1, target: nil, action: nil)
    private let playPause = NSButton()
    private let shuffle = NSButton()
    private let repeatButton = NSButton()
    /// While the seek knob is dragged, the timer leaves it alone.
    private var seekHoldUntil = Date.distantPast
    private var timer: Timer?
    private var artKey: String?
    private var cachedArt: NSImage?

    init(controller: MainWindowController, player: PlayerController) {
        self.controller = controller
        self.player = player
        super.init()
        let h = MiniPlayer.height
        // Frames are set from the top edge, so the layout reads like the design.
        func y(_ top: CGFloat, _ height: CGFloat) -> CGFloat { h - top - height }
        func centred(_ height: CGFloat) -> CGFloat { h - MiniPlayer.transportTop - height / 2 }

        artwork.frame = NSRect(x: 16, y: y(16, 84), width: 84, height: 84)
        artwork.imageScaling = .scaleProportionallyUpOrDown
        artwork.wantsLayer = true
        artwork.layer?.cornerRadius = 10
        artwork.layer?.masksToBounds = true
        artwork.contentTintColor = .secondaryLabelColor

        titleLabel.frame = NSRect(x: 116, y: y(16, 22), width: 208, height: 22)
        titleLabel.font = .systemFont(ofSize: 15, weight: .semibold)
        detailLabel.frame = NSRect(x: 116, y: y(40, 16), width: 208, height: 16)
        detailLabel.font = .systemFont(ofSize: 12)
        detailLabel.textColor = .secondaryLabelColor
        titleLabel.lineBreakMode = .byTruncatingTail
        detailLabel.lineBreakMode = .byTruncatingTail

        seek.frame = NSRect(x: 112, y: y(64, 18), width: 216, height: 18)
        seek.controlSize = .small
        seek.isContinuous = true
        seek.target = self
        seek.action = #selector(seekMoved(_:))
        elapsedLabel.frame = NSRect(x: 116, y: y(84, 14), width: 100, height: 14)
        totalLabel.frame = NSRect(x: 224, y: y(84, 14), width: 100, height: 14)
        totalLabel.alignment = .right
        for l in [elapsedLabel, totalLabel] {
            l.font = .monospacedDigitSystemFont(ofSize: 11, weight: .regular)
            l.textColor = .secondaryLabelColor
        }

        shuffle.frame = NSRect(x: 24, y: centred(30), width: 30, height: 30)
        configureIcon(shuffle, "shuffle", 15, action: #selector(MainWindowController.toggleShuffle))
        let prev = NSButton(frame: NSRect(x: 88, y: centred(36), width: 36, height: 36))
        configureIcon(prev, "backward.end.fill", 18, action: #selector(MainWindowController.previousTrack))
        playPause.frame = NSRect(x: 142, y: centred(56), width: 56, height: 56)
        configureIcon(playPause, "play.fill", 22, action: #selector(MainWindowController.playPausePressed))
        playPause.wantsLayer = true
        playPause.layer?.cornerRadius = 28
        playPause.layer?.backgroundColor = NSColor.controlAccentColor.cgColor
        playPause.contentTintColor = .white
        let next = NSButton(frame: NSRect(x: 216, y: centred(36), width: 36, height: 36))
        configureIcon(next, "forward.end.fill", 18, action: #selector(MainWindowController.nextTrack))
        repeatButton.frame = NSRect(x: 286, y: centred(30), width: 30, height: 30)
        configureIcon(repeatButton, "repeat", 15, action: #selector(MainWindowController.cycleRepeat))

        let lowSpeaker = NSImageView(frame: NSRect(x: 24, y: h - MiniPlayer.volumeTop - 9, width: 18, height: 18))
        lowSpeaker.image = icon("speaker.fill", 11)
        lowSpeaker.contentTintColor = .secondaryLabelColor
        volume.frame = NSRect(x: 50, y: h - MiniPlayer.volumeTop - 9, width: 240, height: 18)
        volume.controlSize = .small
        volume.isContinuous = true
        volume.target = self
        volume.action = #selector(volumeMoved(_:))
        let highSpeaker = NSImageView(frame: NSRect(x: 298, y: h - MiniPlayer.volumeTop - 9, width: 20, height: 18))
        highSpeaker.image = icon("speaker.wave.3.fill", 12)
        highSpeaker.contentTintColor = .secondaryLabelColor

        for v in [artwork, titleLabel, detailLabel, seek, elapsedLabel, totalLabel, shuffle, prev, playPause, next,
                  repeatButton, lowSpeaker, volume, highSpeaker] {
            view.addSubview(v)
        }
        update()
    }

    private func icon(_ name: String, _ size: CGFloat, weight: NSFont.Weight = .medium) -> NSImage? {
        NSImage(systemSymbolName: name, accessibilityDescription: nil)?
            .withSymbolConfiguration(NSImage.SymbolConfiguration(pointSize: size, weight: weight))
    }

    private func configureIcon(_ b: NSButton, _ name: String, _ size: CGFloat, action: Selector) {
        b.isBordered = false
        b.imagePosition = .imageOnly
        b.image = icon(name, size)
        b.contentTintColor = .labelColor
        b.target = controller
        b.action = action
    }

    /// Track details on one line: artist, album, year.
    private func detailText(_ track: Track) -> String {
        let tags = track.info?.tags
        return [tags?.artist, tags?.album, tags?.year].compactMap { $0 }.filter { !$0.isEmpty }.joined(separator: " \u{2022} ")
    }

    /// Copy the player state into the labels, controls and artwork.
    func update() {
        let track = player.heardTrack
        let d = player.duration ?? 0
        titleLabel.stringValue = track?.display ?? "1by1"
        detailLabel.stringValue = track.map(detailText) ?? "Nothing playing"
        elapsedLabel.stringValue = track == nil ? "0:00" : clock(player.position)
        totalLabel.stringValue = track == nil ? "0:00" : (d > 0 ? clock(d) : "--:--")
        seek.isEnabled = track != nil && d > 0
        if Date() > seekHoldUntil {
            seek.doubleValue = d > 0 ? min(1, player.position / d) : 0
        }
        volume.doubleValue = Double(player.settings.volume)
        playPause.image = icon(player.isPlaying ? "pause.fill" : "play.fill", 22, weight: .bold)
        shuffle.contentTintColor = player.shuffle ? .controlAccentColor : .labelColor
        repeatButton.image = icon(player.repeatMode == .one ? "repeat.1" : "repeat", 15)
        repeatButton.contentTintColor = player.repeatMode == .off ? .labelColor : .controlAccentColor
        setArtwork(track)
    }

    /// Art is looked up when the track changes, not on every tick.
    private func setArtwork(_ track: Track?) {
        let key = track?.url.path
        if key != artKey {
            artKey = key
            cachedArt = track.flatMap { albumArt(for: $0.url, names: player.settings.coverNames) }
        }
        artwork.image = cachedArt ?? icon("music.note", 32)
    }

    /// Drag moves the knob. The seek happens once, on mouse release, so the engine is not rebuilt per step.
    @objc private func seekMoved(_ sender: NSSlider) {
        seekHoldUntil = Date().addingTimeInterval(0.8)
        guard NSApp.currentEvent?.type == .leftMouseUp, let d = player.duration, d > 0 else { return }
        player.seek(to: sender.doubleValue * d)
    }

    @objc private func volumeMoved(_ sender: NSSlider) {
        player.setVolume(Float(sender.doubleValue))
    }

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
