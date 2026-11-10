import AppKit

/// Finder-style search: a magnifier button that opens a search field beside it. The field folds back
/// to the button when it is empty and loses focus.
final class SearchToggle: NSView, NSSearchFieldDelegate {
    static let collapsedWidth: CGFloat = 34
    static let expandedWidth: CGFloat = 240
    let field = NSSearchField()
    /// Tells the toolbar item that the size changed.
    var onResize: (() -> Void)?
    private let button = NSButton()
    private(set) var isExpanded = false

    init(target: AnyObject?, action: Selector) {
        super.init(frame: NSRect(x: 0, y: 0, width: Self.collapsedWidth, height: 28))
        button.bezelStyle = .texturedRounded
        button.image = NSImage(systemSymbolName: "magnifyingglass", accessibilityDescription: "Search")
        button.imagePosition = .imageOnly
        button.target = self
        button.action = #selector(buttonClicked)
        button.translatesAutoresizingMaskIntoConstraints = false

        field.placeholderString = "Search"
        field.target = target
        field.action = action
        field.delegate = self
        field.isHidden = true
        field.translatesAutoresizingMaskIntoConstraints = false

        addSubview(button)
        addSubview(field)
        NSLayoutConstraint.activate([
            button.leadingAnchor.constraint(equalTo: leadingAnchor),
            button.centerYAnchor.constraint(equalTo: centerYAnchor),
            button.widthAnchor.constraint(equalToConstant: Self.collapsedWidth),
            button.heightAnchor.constraint(equalToConstant: 28),
            field.leadingAnchor.constraint(equalTo: button.trailingAnchor, constant: 6),
            field.centerYAnchor.constraint(equalTo: centerYAnchor),
            field.trailingAnchor.constraint(equalTo: trailingAnchor),
            field.widthAnchor.constraint(equalToConstant: 200),
        ])
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    override var intrinsicContentSize: NSSize {
        NSSize(width: isExpanded ? Self.expandedWidth : Self.collapsedWidth, height: 28)
    }

    @objc private func buttonClicked() { expand() }

    /// Open the field and put the cursor in it.
    func expand() {
        if !isExpanded {
            isExpanded = true
            field.isHidden = false
            invalidateIntrinsicContentSize()
            onResize?()
        }
        window?.makeFirstResponder(field)
    }

    private func collapse() {
        isExpanded = false
        field.isHidden = true
        invalidateIntrinsicContentSize()
        onResize?()
    }

    func controlTextDidEndEditing(_ obj: Notification) {
        if field.stringValue.isEmpty { collapse() }
    }
}
