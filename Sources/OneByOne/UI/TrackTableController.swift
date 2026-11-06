import AppKit

/// Track list in an NSTableView. Rows come from the player's current folder.
final class TrackTableController: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    let view: NSScrollView
    let table = NSTableView()
    private var rows: [Track] = []
    private var heardRow: Int?
    var onActivate: ((Int) -> Void)?
    var onSelectionChange: (() -> Void)?
    /// Header click: column key and direction.
    var onSort: ((SortKey, Bool) -> Void)?
    /// Size column shows length too unless this is set.
    var sizeWithoutLength = false

    init(autosaveName: String) {
        view = NSScrollView()
        super.init()
        let columns: [(String, String, CGFloat)] = [
            ("name", "Name", 300), ("length", "Length", 70), ("size", "Size", 80),
            ("type", "Type", 60), ("info", "Info", 240),
        ]
        for (id, title, width) in columns {
            let c = NSTableColumn(identifier: NSUserInterfaceItemIdentifier(id))
            c.title = title
            c.width = width
            c.minWidth = 40
            if let key = SortKey(rawValue: id) {
                c.sortDescriptorPrototype = NSSortDescriptor(key: key.rawValue, ascending: true)
            }
            table.addTableColumn(c)
        }
        table.dataSource = self
        table.delegate = self
        table.usesAlternatingRowBackgroundColors = true
        table.allowsMultipleSelection = true
        table.target = self
        table.doubleAction = #selector(activated)
        table.rowHeight = 20
        table.autosaveName = autosaveName
        table.autosaveTableColumns = true
        view.documentView = table
        view.hasVerticalScroller = true
        view.hasHorizontalScroller = true
        view.autohidesScrollers = true
    }

    var selectedRows: IndexSet { table.selectedRowIndexes }

    var selectedRowsIfAny: IndexSet? {
        table.selectedRowIndexes.isEmpty ? nil : table.selectedRowIndexes
    }

    var selectedRow: Int? {
        let r = table.selectedRow
        return r >= 0 ? r : nil
    }

    func setTracks(_ tracks: [Track]) {
        rows = tracks
        table.reloadData()
    }

    func reload() {
        table.reloadData()
    }

    func scrollToRow(_ row: Int) {
        table.scrollRowToVisible(row)
    }

    func select(row: Int) {
        table.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
        table.scrollRowToVisible(row)
    }

    func setHeard(_ row: Int?) {
        let old = heardRow
        heardRow = row
        for r in [old, row].compactMap({ $0 }) where r < rows.count {
            table.reloadData(forRowIndexes: IndexSet(integer: r), columnIndexes: IndexSet(integersIn: 0..<table.numberOfColumns))
        }
    }

    @objc private func activated() {
        let r = table.clickedRow
        if r >= 0 { onActivate?(r) }
    }

    // MARK: NSTableViewDataSource

    func numberOfRows(in tableView: NSTableView) -> Int { rows.count }

    // MARK: NSTableViewDelegate

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        guard let col = tableColumn, rows.indices.contains(row) else { return nil }
        let t = rows[row]
        var text: String
        var alignRight = false
        switch col.identifier.rawValue {
        case "name": text = t.display
        case "length": text = t.duration.map { clock($0) } ?? ""; alignRight = true
        case "size":
            text = sizeText(t.size)
            if !sizeWithoutLength, let d = t.duration { text += "  " + clock(d) }
            alignRight = true
        case "type": text = t.typeText
        case "info": text = t.infoText
        default: text = ""
        }
        let id = NSUserInterfaceItemIdentifier("cell.\(col.identifier.rawValue)")
        let cell = tableView.makeView(withIdentifier: id, owner: self) as? NSTableCellView ?? makeCell(id)
        cell.textField?.stringValue = text
        cell.textField?.alignment = alignRight ? .right : .left
        let heard = row == heardRow
        cell.textField?.font = heard ? .boldSystemFont(ofSize: 13) : .systemFont(ofSize: 13)
        // On a selected row the accent blue would vanish into the selection blue, so use white there.
        let selected = table.isRowSelected(row)
        cell.textField?.textColor = selected ? .alternateSelectedControlTextColor
            : heard ? .controlAccentColor : .labelColor
        return cell
    }

    private func makeCell(_ id: NSUserInterfaceItemIdentifier) -> NSTableCellView {
        let c = NSTableCellView()
        c.identifier = id
        let tf = NSTextField(labelWithString: "")
        tf.lineBreakMode = .byTruncatingTail
        tf.translatesAutoresizingMaskIntoConstraints = false
        c.addSubview(tf)
        c.textField = tf
        NSLayoutConstraint.activate([
            tf.leadingAnchor.constraint(equalTo: c.leadingAnchor, constant: 4),
            tf.trailingAnchor.constraint(equalTo: c.trailingAnchor, constant: -4),
            tf.centerYAnchor.constraint(equalTo: c.centerYAnchor),
        ])
        return c
    }

    func tableViewSelectionDidChange(_ notification: Notification) {
        // Repaint the now-playing row so its colour follows the selection.
        if let r = heardRow, r < rows.count {
            table.reloadData(forRowIndexes: IndexSet(integer: r),
                             columnIndexes: IndexSet(integersIn: 0..<table.numberOfColumns))
        }
        onSelectionChange?()
    }

    func tableView(_ tableView: NSTableView, sortDescriptorsDidChange oldDescriptors: [NSSortDescriptor]) {
        guard let d = tableView.sortDescriptors.first, let key = d.key.flatMap(SortKey.init(rawValue:)) else { return }
        onSort?(key, !d.ascending)
    }
}

extension TrackTableController {
    /// Grid, scrollbars, row height and the info column, as set in settings.
    func applyStyle(grid: Bool, hideScrollbars: Bool, rowHeight: CGFloat, showInfo: Bool) {
        table.gridStyleMask = grid ? [.solidVerticalGridLineMask, .solidHorizontalGridLineMask] : []
        table.rowHeight = rowHeight
        view.hasVerticalScroller = !hideScrollbars
        view.hasHorizontalScroller = !hideScrollbars
        table.tableColumns.first { $0.identifier.rawValue == "info" }?.isHidden = !showInfo
        table.reloadData()
    }
}
