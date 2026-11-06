import AppKit

/// Large title, times and info for the heard track. The Rust build's "Big view".
/// Rows follow the chosen layout. The info row cycles through the pattern tokens.
final class BigTitleWindow: NSWindowController {
    private let stack = NSStackView()
    private let title = NSTextField(labelWithString: "")
    private let times = NSTextField(labelWithString: "")
    private let info = NSTextField(labelWithString: "")
    private let info2 = NSTextField(labelWithString: "")
    private var cycleIndex = 0
    private var lastCycle = Date.distantPast

    init() {
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 240),
                           styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        win.title = "1by1 - Big View"
        win.minSize = NSSize(width: 320, height: 140)
        super.init(window: win)
        title.font = .systemFont(ofSize: 40, weight: .semibold)
        title.lineBreakMode = .byTruncatingTail
        times.font = .monospacedDigitSystemFont(ofSize: 28, weight: .regular)
        times.textColor = .secondaryLabelColor
        for f in [info, info2] {
            f.font = .systemFont(ofSize: 14)
            f.textColor = .secondaryLabelColor
            f.lineBreakMode = .byTruncatingTail
        }
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.translatesAutoresizingMaskIntoConstraints = false
        win.contentView?.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: win.contentView!.leadingAnchor, constant: 24),
            stack.trailingAnchor.constraint(lessThanOrEqualTo: win.contentView!.trailingAnchor, constant: -24),
            stack.topAnchor.constraint(equalTo: win.contentView!.topAnchor, constant: 20),
            stack.bottomAnchor.constraint(lessThanOrEqualTo: win.contentView!.bottomAnchor, constant: -20),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// Update the rows for `track`, its position and length.
    func update(track: Track?, position: Double, duration: Double?, layout: BigLayout,
                pattern: String, padding: Int, cycleSeconds: Int) {
        stack.edgeInsets = NSEdgeInsets(top: 0, left: 0, bottom: 0, right: 0)
        stack.spacing = CGFloat(max(4, padding))
        guard let track else {
            title.stringValue = "Stopped"
            times.stringValue = ""
            info.stringValue = ""
            info2.stringValue = ""
            setRows([title, times])
            return
        }
        title.stringValue = track.artistTitle
        times.stringValue = "\(clock(position)) / \(duration.map { clock($0) } ?? "--:--")"
        let tokens = pattern.split(separator: "|").map { $0.trimmingCharacters(in: .whitespaces).uppercased() }
        let infoTokens = tokens.filter { $0 != "TITLE" && $0 != "TIME" }
        let infoText = infoTokens.isEmpty ? track.infoText : infoLine(track, infoTokens[cycleIndex % infoTokens.count])
        info.stringValue = infoText
        info2.stringValue = track.info?.format ?? track.typeText
        if cycleSeconds > 0, infoTokens.count > 1, Date().timeIntervalSince(lastCycle) >= Double(cycleSeconds) {
            cycleIndex += 1
            lastCycle = Date()
        }
        switch layout {
        case .titleTimes: setRows([title, times])
        case .titleTimesInfo: setRows([title, times, info])
        case .titleInfo2: setRows([title, info, info2, times])
        case .infoTitleTimes: setRows([info, title, times, info2])
        }
    }

    private func setRows(_ rows: [NSView]) {
        stack.arrangedSubviews.forEach { $0.removeFromSuperview() }
        rows.forEach { stack.addArrangedSubview($0) }
    }

    private func infoLine(_ t: Track, _ token: String) -> String {
        switch token {
        case "INFO": return t.info?.summary ?? ""
        case "ARTIST": return t.info?.tags.artist ?? ""
        case "ALBUM": return t.info?.tags.album ?? ""
        case "FORMAT": return t.info?.format ?? t.info?.decoder ?? ""
        default: return token
        }
    }
}
