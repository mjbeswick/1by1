import AppKit

/// Searches audio file names under the folder tree roots. Double-click plays the file.
final class FindFilesWindow: NSWindowController, NSTableViewDataSource, NSTableViewDelegate, NSSearchFieldDelegate {
    private let roots: [URL]
    private let field = NSSearchField()
    private let table = NSTableView()
    private var hits: [URL] = []
    var onPick: ((URL) -> Void)?
    private var generation = 0

    init(roots: [URL]) {
        self.roots = roots
        let win = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 560, height: 420),
                           styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        win.title = "Find Files"
        super.init(window: win)
        field.placeholderString = "Search file names"
        field.delegate = self
        field.target = self
        field.action = #selector(searchChanged)
        let name = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("name"))
        name.title = "File"
        name.width = 240
        let path = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("path"))
        path.title = "Folder"
        path.width = 280
        table.addTableColumn(name)
        table.addTableColumn(path)
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.doubleAction = #selector(pick)
        let scroll = NSScrollView()
        scroll.documentView = table
        scroll.hasVerticalScroller = true
        let stack = NSStackView(views: [field, scroll])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.edgeInsets = NSEdgeInsets(top: 12, left: 12, bottom: 12, right: 12)
        stack.translatesAutoresizingMaskIntoConstraints = false
        scroll.translatesAutoresizingMaskIntoConstraints = false
        field.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -24).isActive = true
        scroll.widthAnchor.constraint(equalTo: stack.widthAnchor, constant: -24).isActive = true
        scroll.heightAnchor.constraint(greaterThanOrEqualToConstant: 300).isActive = true
        win.contentView?.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: win.contentView!.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: win.contentView!.trailingAnchor),
            stack.topAnchor.constraint(equalTo: win.contentView!.topAnchor),
            stack.bottomAnchor.constraint(equalTo: win.contentView!.bottomAnchor),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func controlTextDidChange(_ obj: Notification) { searchChanged() }

    /// Walk the roots on a background queue. Newer searches cancel older ones.
    @objc private func searchChanged() {
        let text = field.stringValue
        generation += 1
        let gen = generation
        guard !text.isEmpty else { hits = []; table.reloadData(); return }
        DispatchQueue.global(qos: .userInitiated).async { [roots] in
            var found: [URL] = []
            for root in roots {
                guard let e = FileManager.default.enumerator(at: root, includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey],
                                                             options: [.skipsHiddenFiles]) else { continue }
                for case let url as URL in e {
                    if found.count >= 500 { break }
                    let v = try? url.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])
                    if v?.isSymbolicLink == true { e.skipDescendants(); continue }
                    if v?.isDirectory == false, isAudio(url), containsCI(url.lastPathComponent, text) {
                        found.append(url)
                    }
                }
            }
            DispatchQueue.main.async { [weak self] in
                guard let self, gen == self.generation else { return }
                self.hits = found
                self.table.reloadData()
            }
        }
    }

    @objc private func pick() {
        let r = table.clickedRow
        guard hits.indices.contains(r) else { return }
        onPick?(hits[r])
    }

    func numberOfRows(in tableView: NSTableView) -> Int { hits.count }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let url = hits[row]
        let text = tableColumn?.identifier.rawValue == "name" ? url.lastPathComponent : url.deletingLastPathComponent().path
        return NSTextField(labelWithString: text)
    }
}
