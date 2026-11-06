import AppKit

/// One folder in the tree. Children load on first expand.
final class FolderNode {
    let url: URL
    private var cached: [FolderNode]?

    init(url: URL) { self.url = url }

    var children: [FolderNode] {
        if let cached { return cached }
        let kids = subfolders(url).map(FolderNode.init(url:))
        cached = kids
        return kids
    }

    var name: String { url.lastPathComponent }
}

/// Folder tree in an NSOutlineView. Lazy children, single selection.
final class FolderTreeController: NSObject, NSOutlineViewDataSource, NSOutlineViewDelegate {
    let view: NSScrollView
    let outline = NSOutlineView()
    private(set) var roots: [URL] = []
    private var rootNodes: [FolderNode] = []
    var onSelect: ((URL) -> Void)?

    override init() {
        view = NSScrollView()
        super.init()
        let col = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("folder"))
        col.title = "Folders"
        outline.addTableColumn(col)
        outline.outlineTableColumn = col
        outline.headerView = nil
        outline.rowSizeStyle = .default
        outline.style = .sourceList
        outline.dataSource = self
        outline.delegate = self
        outline.target = self
        outline.action = #selector(clicked)
        view.documentView = outline
        view.hasVerticalScroller = true
        view.drawsBackground = false
    }

    /// Explorer style uses the inset list look instead of the source list.
    func setExplorerStyle(_ on: Bool) {
        outline.style = on ? .inset : .sourceList
    }

    /// Indent and row height, from the window style settings.
    func applyStyle(rowSpacing: Bool, bigger: Bool, lines: Bool) {
        outline.indentationPerLevel = rowSpacing ? 18 : 12
        outline.rowHeight = bigger ? 24 : 18
        outline.gridStyleMask = lines ? [.solidHorizontalGridLineMask] : []
    }

    func setRoots(_ urls: [URL]) {
        roots = urls
        rootNodes = urls.map(FolderNode.init(url:))
        outline.reloadData()
        for n in rootNodes { outline.expandItem(n) }
    }

    func add(root url: URL) {
        guard !roots.contains(url) else { return }
        roots.append(url)
        rootNodes.append(FolderNode(url: url))
        outline.reloadData()
        outline.expandItem(rootNodes.last)
    }

    /// Select `url` in the tree if it is visible.
    func select(_ url: URL) {
        // Expand each ancestor on the way down, so the target row exists.
        var level = rootNodes
        while let node = level.first(where: { url.path == $0.url.path || url.path.hasPrefix($0.url.path + "/") }),
              node.url != url {
            outline.expandItem(node)
            level = node.children
        }
        for row in 0..<outline.numberOfRows {
            if let n = outline.item(atRow: row) as? FolderNode, n.url == url {
                outline.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
                outline.scrollRowToVisible(row)
                return
            }
        }
    }

    @objc private func clicked() {
        guard let n = outline.item(atRow: outline.clickedRow) as? FolderNode else { return }
        onSelect?(n.url)
    }

    // MARK: NSOutlineViewDataSource

    func outlineView(_ outlineView: NSOutlineView, numberOfChildrenOfItem item: Any?) -> Int {
        item == nil ? rootNodes.count : (item as? FolderNode)?.children.count ?? 0
    }

    func outlineView(_ outlineView: NSOutlineView, child index: Int, ofItem item: Any?) -> Any {
        item == nil ? rootNodes[index] : (item as! FolderNode).children[index]
    }

    func outlineView(_ outlineView: NSOutlineView, isItemExpandable item: Any) -> Bool {
        !(item as! FolderNode).children.isEmpty
    }

    // MARK: NSOutlineViewDelegate

    func outlineView(_ outlineView: NSOutlineView, viewFor tableColumn: NSTableColumn?, item: Any) -> NSView? {
        let node = item as! FolderNode
        let id = NSUserInterfaceItemIdentifier("cell")
        let cell = outlineView.makeView(withIdentifier: id, owner: self) as? NSTableCellView ?? {
            let c = NSTableCellView()
            c.identifier = id
            let tf = NSTextField(labelWithString: "")
            tf.lineBreakMode = .byTruncatingTail
            tf.translatesAutoresizingMaskIntoConstraints = false
            c.addSubview(tf)
            c.textField = tf
            NSLayoutConstraint.activate([
                tf.leadingAnchor.constraint(equalTo: c.leadingAnchor, constant: 2),
                tf.trailingAnchor.constraint(equalTo: c.trailingAnchor, constant: -4),
                tf.centerYAnchor.constraint(equalTo: c.centerYAnchor),
            ])
            return c
        }()
        cell.textField?.stringValue = node.name
        return cell
    }

    func outlineViewSelectionDidChange(_ notification: Notification) {
        guard let n = outline.item(atRow: outline.selectedRow) as? FolderNode else { return }
        onSelect?(n.url)
    }
}
