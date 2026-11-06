import AppKit

/// Large title and times for the heard track. The Rust build's "Big view".
final class BigTitleWindow: NSWindowController {
    private let title = NSTextField(labelWithString: "")
    private let times = NSTextField(labelWithString: "")
    private let info = NSTextField(labelWithString: "")

    init() {
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 220),
                           styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        win.title = "1by1 - Big View"
        win.minSize = NSSize(width: 320, height: 140)
        super.init(window: win)
        title.font = .systemFont(ofSize: 40, weight: .semibold)
        title.lineBreakMode = .byTruncatingTail
        times.font = .monospacedDigitSystemFont(ofSize: 28, weight: .regular)
        times.textColor = .secondaryLabelColor
        info.font = .systemFont(ofSize: 14)
        info.textColor = .secondaryLabelColor
        info.lineBreakMode = .byTruncatingTail
        let stack = NSStackView(views: [title, times, info])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 24, bottom: 20, right: 24)
        stack.translatesAutoresizingMaskIntoConstraints = false
        win.contentView?.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: win.contentView!.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: win.contentView!.trailingAnchor),
            stack.topAnchor.constraint(equalTo: win.contentView!.topAnchor),
            stack.bottomAnchor.constraint(lessThanOrEqualTo: win.contentView!.bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// Show `track`, its position and length.
    func update(track: Track?, position: Double, duration: Double?) {
        guard let track else {
            title.stringValue = "Stopped"
            times.stringValue = ""
            info.stringValue = ""
            return
        }
        title.stringValue = track.artistTitle
        times.stringValue = "\(clock(position)) / \(duration.map { clock($0) } ?? "--:--")"
        info.stringValue = [track.info?.format ?? track.typeText, track.info?.summary ?? ""]
            .filter { !$0.isEmpty }.joined(separator: "  ·  ")
    }
}
