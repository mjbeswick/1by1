import AppKit

/// Applies a theme: window appearance plus list and tree background.
extension MainWindowController {
    func applyTheme() {
        let t = player.settings.theme
        let dark: Bool
        let bg: NSColor?
        switch t {
        case .current: dark = false; bg = nil
        case .dark: dark = true; bg = nil
        case .anthracite: dark = true; bg = NSColor(srgbRed: 0.17, green: 0.18, blue: 0.19, alpha: 1)
        case .paper: dark = false; bg = NSColor(srgbRed: 0.98, green: 0.97, blue: 0.94, alpha: 1)
        case .grey: dark = false; bg = NSColor(srgbRed: 0.93, green: 0.93, blue: 0.93, alpha: 1)
        case .deepBlue: dark = true; bg = NSColor(srgbRed: 0.05, green: 0.11, blue: 0.23, alpha: 1)
        case .blue: dark = false; bg = NSColor(srgbRed: 0.90, green: 0.94, blue: 1.0, alpha: 1)
        case .red: dark = false; bg = NSColor(srgbRed: 1.0, green: 0.91, blue: 0.91, alpha: 1)
        }
        window?.appearance = t == .current ? nil : NSAppearance(named: dark ? .darkAqua : .aqua)
        for list in [table, playlistTable] {
            list.table.backgroundColor = bg ?? .controlBackgroundColor
        }
        tree.outline.backgroundColor = bg ?? .clear
    }
}
