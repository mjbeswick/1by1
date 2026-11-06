import Foundation

/// Keeps playback indexes valid when the folder list is re-sorted or re-read.
extension PlayerController {
    private func key(_ t: Track) -> String { "\(t.url.path)#\(t.cue?.start ?? -1)" }

    /// Replace the folder list in `newOrder`, same tracks. Playing rows follow their tracks.
    func replaceFolderTracks(_ newOrder: [Track]) {
        let oldHeardKey = heard.flatMap { folderTracks.indices.contains($0) ? key(folderTracks[$0]) : nil }
        let oldLastKey = lastHeard.flatMap { folderTracks.indices.contains($0) ? key(folderTracks[$0]) : nil }
        let oldFeedKey = feedCursor.flatMap { folderTracks.indices.contains($0) ? key(folderTracks[$0]) : nil }
        let index = Dictionary(newOrder.enumerated().map { (key($0.element), $0.offset) }, uniquingKeysWith: { a, _ in a })
        folderTracks = newOrder
        if !usesPlaylist {
            heard = oldHeardKey.flatMap { index[$0] }
            lastHeard = oldLastKey.flatMap { index[$0] }
            feedCursor = oldFeedKey.flatMap { index[$0] }
            fed = fed.compactMap { i in folderTracks.indices.contains(i) ? index[key(folderTracks[i])] : nil }
        }
        onChange?()
    }

    /// Sort the folder list by a column. Playing rows keep following their tracks.
    func sortFolder(by key: SortKey, descending: Bool) {
        let keyed = folderTracks.map { ($0, sortValue($0, key)) }
        let sorted = keyed.sorted { a, b in
            let order: Bool
            switch (a.1, b.1) {
            case let (.text(x), .text(y)): order = naturalCompare(x, y) == .orderedAscending
            case let (.number(x), .number(y)): order = x < y
            default: order = false
            }
            return descending ? !order && !(a.1 == b.1) : order
        }
        settings.sort = key
        settings.sortDesc = descending
        replaceFolderTracks(sorted.map { $0.0 })
    }

    private enum SortValue: Equatable {
        case text(String)
        case number(Double)
    }

    private func sortValue(_ t: Track, _ key: SortKey) -> SortValue {
        switch key {
        case .name: return .text(t.display)
        case .length: return .number(t.duration ?? 0)
        case .size: return .number(Double(t.size))
        case .type: return .text(t.typeText)
        case .modified: return .number(t.modified?.timeIntervalSince1970 ?? 0)
        case .created: return .number(t.created?.timeIntervalSince1970 ?? 0)
        }
    }

    /// Re-read the folder. Adds new files, drops removed ones, keeps the playing track.
    /// Returns true when the list changed.
    @discardableResult
    func refreshFolderIfChanged() -> Bool {
        guard !usesPlaylist, let folder else { return false }
        let fresh = folderRows(folder, withFolders: settings.foldersInList)
        let before = folderTracks.map(key)
        let after = fresh.map(key)
        guard before != after else { return false }
        // Keep info already read for unchanged files.
        let known = Dictionary(folderTracks.map { (key($0), $0) }, uniquingKeysWith: { a, _ in a })
        for t in fresh { if let old = known[key(t)], let info = old.info { t.info = info } }
        replaceFolderTracks(fresh)
        return true
    }
}
