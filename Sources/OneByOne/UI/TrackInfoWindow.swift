import AppKit

/// Tag editor and read-only audio properties for one track. Tracker modules are read-only.
final class TrackInfoWindow: NSWindowController {
    private let track: Track
    private var fields: [String: NSTextField] = [:]
    private let message = NSTextField(labelWithString: "")
    var onSaved: ((Track) -> Void)?

    private static let editable: [(key: String, label: String)] = [
        ("title", "Title"), ("artist", "Artist"), ("album", "Album"),
        ("year", "Year"), ("track", "Track"), ("comment", "Comment"),
    ]

    init(track: Track) {
        self.track = track
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 420, height: 420),
                           styleMask: [.titled, .closable], backing: .buffered, defer: false)
        win.title = track.name
        super.init(window: win)
        win.contentView = buildForm()
        if isModule(track.url), let root = win.contentView {
            win.setContentSize(root.fittingSize)
        }
        loadValues()
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    /// Tracker modules get a read-only module view: structure, tracker and the song message.
    private func buildModuleView() -> NSView {
        let details = moduleDetails(track.url)
        let grid = NSGridView()
        grid.rowSpacing = 6
        grid.columnSpacing = 12
        for (label, value) in details.rows {
            let v = NSTextField(labelWithString: value)
            v.isSelectable = true
            v.lineBreakMode = .byTruncatingTail
            let l = NSTextField(labelWithString: label)
            l.textColor = .secondaryLabelColor
            grid.addRow(with: [l, v])
        }
        grid.column(at: 0).xPlacement = .trailing
        let text = NSTextView()
        text.isEditable = false
        text.isSelectable = true
        text.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        text.string = details.message.isEmpty ? "No song message." : details.message
        text.textContainerInset = NSSize(width: 6, height: 6)
        let scroll = NSScrollView()
        scroll.documentView = text
        scroll.hasVerticalScroller = true
        scroll.borderType = .bezelBorder
        scroll.translatesAutoresizingMaskIntoConstraints = false
        scroll.heightAnchor.constraint(equalToConstant: 150).isActive = true
        scroll.widthAnchor.constraint(equalToConstant: 428).isActive = true
        let close = NSButton(title: "Close", target: self, action: #selector(closeWindow))
        close.keyEquivalent = "\u{1b}"
        let root = NSStackView(views: [grid, NSTextField(labelWithString: "Song message"), scroll, close])
        root.orientation = .vertical
        root.alignment = .leading
        root.spacing = 12
        root.edgeInsets = NSEdgeInsets(top: 16, left: 16, bottom: 16, right: 16)
        return root
    }

    private func buildForm() -> NSView {
        if isModule(track.url) { return buildModuleView() }
        let grid = NSGridView()
        grid.rowSpacing = 8
        grid.columnSpacing = 10
        for (key, label) in Self.editable {
            let f = NSTextField()
            f.placeholderString = label
            f.isEnabled = !isModule(track.url)
            fields[key] = f
            grid.addRow(with: [NSTextField(labelWithString: label), f])
        }
        let info = track.info
        let props = [
            "Format": info?.format ?? info?.decoder ?? "",
            "Length": track.duration.map { clock($0) } ?? "--:--",
            "Properties": info?.summary ?? "",
            "Size": sizeText(track.size),
            "Path": track.url.path,
        ]
        grid.addRow(with: [NSTextField(labelWithString: " "), NSTextField(labelWithString: " ")])
        for key in ["Format", "Length", "Properties", "Size", "Path"] {
            let value = NSTextField(wrappingLabelWithString: props[key] ?? "")
            value.textColor = .secondaryLabelColor
            value.isSelectable = true
            grid.addRow(with: [NSTextField(labelWithString: key), value])
        }
        message.textColor = .secondaryLabelColor
        message.lineBreakMode = .byWordWrapping
        let save = NSButton(title: "Save", target: self, action: #selector(save))
        save.keyEquivalent = "\r"
        save.isEnabled = !isModule(track.url)
        let close = NSButton(title: "Close", target: self, action: #selector(closeWindow))
        close.keyEquivalent = "\u{1b}"
        let buttons = NSStackView(views: [message, close, save])
        buttons.spacing = 8

        let root = NSStackView(views: [grid, buttons])
        root.orientation = .vertical
        root.alignment = .leading
        root.spacing = 16
        root.edgeInsets = NSEdgeInsets(top: 16, left: 16, bottom: 16, right: 16)
        root.translatesAutoresizingMaskIntoConstraints = true
        root.frame = NSRect(x: 0, y: 0, width: 420, height: 420)
        return root
    }

    private func loadValues() {
        let tags = track.info?.tags ?? Tags()
        let values: [String: String?] = ["title": tags.title, "artist": tags.artist, "album": tags.album,
                      "year": tags.year, "track": tags.track, "comment": tags.comment]
        for (key, field) in fields {
            field.stringValue = values[key].flatMap { $0 } ?? ""
        }
        if isModule(track.url) { message.stringValue = "Tracker modules store no editable tags." }
    }

    @objc private func save() {
        let tags = Tags(
            title: fields["title"]?.stringValue, artist: fields["artist"]?.stringValue,
            album: fields["album"]?.stringValue, year: fields["year"]?.stringValue,
            track: fields["track"]?.stringValue, comment: fields["comment"]?.stringValue)
        switch writeTags(track.url, tags) {
        case .success:
            track.info?.tags = tags
            message.stringValue = "Saved."
            onSaved?(track)
        case .failure(let error):
            message.stringValue = "\(error)"
        }
    }

    @objc private func closeWindow() {
        window?.close()
    }
}
