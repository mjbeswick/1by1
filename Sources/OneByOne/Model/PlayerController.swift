import Foundation

/// Owns the folder and playlist lists and drives the engine. Main thread only.
final class PlayerController {
    let engine = Engine()
    var settings: Settings

    var folder: URL?
    var folderTracks: [Track] = []
    /// Queued tracks from any folder. Played in order when `usesPlaylist` is set.
    var playlist: [Track] = []
    var usesPlaylist = false

    /// The list playback walks: the playlist or the current folder.
    var tracks: [Track] { usesPlaylist ? playlist : folderTracks }
    /// Row being heard. `nil` when stopped.
    var heard: Int?
    /// Last row heard. Kept after the track ends, for end-of-track handling.
    var lastHeard: Int?
    /// Rows handed to the engine and not yet heard. The head is heard next.
    var fed: [Int] = []
    /// Row the gapless feeder is on.
    var feedCursor: Int?
    /// Saved row and position for Space after a restart. Cleared when playback starts.
    var pendingResume: (row: Int, seconds: Double)?
    /// Intro scan: seconds per track and when the current one started.
    var intro: (perTrack: Double, started: Date)?

    /// Set by Quit After Track. The window quits when the next track would start.
    var quitAfterTrack = false
    /// The app should quit now (quit-after-track fired).
    var onQuitRequested: (() -> Void)?

    var repeatMode: Repeat { didSet { settings.repeatMode = repeatMode; save() } }
    var shuffle: Bool { didSet { settings.shuffle = shuffle; save() } }

    /// Any state change the UI should show.
    var onChange: (() -> Void)?
    /// Short status line for the window.
    var onMessage: ((String) -> Void)?
    /// A track became the heard one. The window uses this to follow the selection.
    var onTrackHeard: ((Int) -> Void)?
    /// Folder finished with repeat off. The window reloads or switches folder.
    var onFolderEnd: (() -> Void)?

    init(settings: Settings) {
        self.settings = settings
        repeatMode = settings.repeatMode
        shuffle = settings.shuffle
        engine.volume = settings.muted ? 0 : settings.volume
        engine.nextProvider = { [weak self] in self?.nextFeedSpec() }
        engine.onTrackChanged = { [weak self] _ in self?.trackChanged() }
        engine.onFinished = { [weak self] in self?.finished() }
        engine.onError = { [weak self] spec, msg in
            self?.onMessage?("Cannot play \(spec.url.lastPathComponent): \(msg)")
        }
    }

    var isPlaying: Bool { engine.state == .playing }
    var isPaused: Bool { engine.state == .paused }
    var position: Double {
        if engine.state == .stopped, let p = pendingResume { return p.seconds }
        return engine.position
    }
    var duration: Double? { heard.flatMap { tracks.indices.contains($0) ? tracks[$0].duration : nil } }
    var heardTrack: Track? { heard.flatMap { tracks.indices.contains($0) ? tracks[$0] : nil } }

    func save() { settings.save() }

    // MARK: - Folder

    /// Replace the list with `tracks` from `folder`. Playback stops, since list indexes change.
    func setFolder(_ url: URL, tracks newTracks: [Track]) {
        if engine.state != .stopped { stop() }
        folder = url
        folderTracks = newTracks
        usesPlaylist = false
        settings.lastFolder = url.path
        pendingResume = nil
        if settings.rememberPositions, let r = settings.folderResume[url.path],
           let row = newTracks.firstIndex(where: { $0.url.path == r.path }) {
            pendingResume = (row, r.seconds)
        }
        save()
        onChange?()
    }

    /// Row to select after `setFolder`, from the saved resume point.
    var resumeRow: Int? { pendingResume?.row }

    // MARK: - Playlist

    /// Append folder rows to the playlist. Returns how many were added.
    @discardableResult
    func enqueue(rows: [Int]) -> Int {
        let add = rows.filter { folderTracks.indices.contains($0) }.map { folderTracks[$0] }
        playlist += add
        onChange?()
        return add.count
    }

    func removePlaylistRows(_ rows: IndexSet) {
        playlist = playlist.enumerated().filter { !rows.contains($0.offset) }.map { $0.element }
        onChange?()
    }

    func clearPlaylist() {
        playlist = []
        onChange?()
    }

    /// Play a row of the current folder.
    func playFolder(row: Int) {
        usesPlaylist = false
        play(row: row)
    }

    /// Play a row of the playlist.
    func playFromPlaylist(row: Int) {
        guard playlist.indices.contains(row) else { return }
        usesPlaylist = true
        play(row: row)
    }

    // MARK: - Transport

    /// Play a row from its start, or from `seconds` into it.
    func play(row: Int, at seconds: Double = 0) {
        guard tracks.indices.contains(row) else { return }
        pendingResume = nil
        fed = [row]
        feedCursor = row
        engine.play(tracks[row].spec, at: seconds)
    }

    /// Space: play or pause. Stopped resumes the saved point, else plays the given row.
    func togglePause(selectedRow: Int?) {
        switch engine.state {
        case .playing: engine.pause()
        case .paused: engine.resume()
        case .stopped:
            if let p = pendingResume, tracks.indices.contains(p.row) {
                play(row: p.row, at: max(0, p.seconds - Double(settings.resumeBackMs) / 1000))
            } else {
                play(row: selectedRow ?? lastHeard ?? 0)
            }
        }
        onChange?()
    }

    func stop() {
        rememberResume()
        engine.stop()
        heard = nil
        fed = []
        feedCursor = nil
        intro = nil
        onChange?()
    }

    func next() {
        guard let cur = heard ?? feedCursor ?? lastHeard,
              let n = nextIndex(count: tracks.count, cur: cur, repeat: repeatMode) else {
            stop()
            return
        }
        play(row: n)
    }

    func prev() {
        guard let cur = heard ?? feedCursor ?? lastHeard else { return }
        play(row: prevIndex(count: tracks.count, cur: cur, repeat: repeatMode))
    }

    /// Seek inside the heard track. Rebuilds the gapless queue after the new position.
    func seek(to seconds: Double) {
        guard let row = heard else { return }
        fed = [row]
        feedCursor = row
        engine.seek(to: seconds)
        onChange?()
    }

    /// Move by `seconds`. Past the end, Next-at-end-of-forward goes to the next track.
    func rewind(by seconds: Double) {
        let target = position + seconds
        if let d = duration, target >= d, settings.nextAtEndOfFFwd, seconds > 0 {
            next()
            return
        }
        seek(to: max(0, target))
    }

    func setVolume(_ v: Float) {
        settings.volume = max(0, min(1, v))
        settings.muted = false
        applyVolume()
    }

    func toggleMute() {
        settings.muted.toggle()
        applyVolume()
    }

    private func applyVolume() {
        engine.volume = settings.muted ? 0 : settings.volume
        save()
        onChange?()
    }

    // MARK: - Intro scan

    func startIntro(perTrack: Double = 10) {
        intro = (perTrack, Date())
        if !isPlaying { togglePause(selectedRow: lastHeard) }
        onChange?()
    }

    func stopIntro() {
        intro = nil
        onChange?()
    }

    /// Called from the window's clock. Moves on when a track's intro has played.
    func tickIntro() {
        guard let i = intro, Date().timeIntervalSince(i.started) >= i.perTrack else { return }
        intro = (i.perTrack, Date())
        next()
    }

    // MARK: - Engine callbacks

    /// The engine started hearing the head of its queue.
    private func trackChanged() {
        guard !fed.isEmpty else { return }
        let row = fed.removeFirst()
        if quitAfterTrack, heard != nil {
            quitAfterTrack = false
            engine.stop()
            onQuitRequested?()
            return
        }
        heard = row
        lastHeard = row
        if let t = heardTrack {
            if settings.logEnabled { TrackLog.append(t, maxKB: settings.logMaxKB) }
            rememberRecentTrack(t)
        }
        rememberResume()
        onTrackHeard?(row)
        onChange?()
    }

    private func finished() {
        heard = nil
        fed = []
        feedCursor = nil
        onChange?()
        // Without gapless the next track starts here, after a short gap.
        if !settings.gapless, let last = lastHeard, settings.oneTrack == false,
           let n = nextIndex(count: tracks.count, cur: last, repeat: repeatMode) {
            play(row: n)
            return
        }
        if lastHeard != nil, !settings.oneTrack, repeatMode == .off,
           lastHeard == tracks.count - 1 {
            onFolderEnd?()
        }
    }

    /// Next row for the gapless feeder. Skips files that will not open.
    private func nextFeedSpec() -> Spec? {
        guard let cur = feedCursor, !tracks.isEmpty, settings.gapless, !settings.oneTrack else { return nil }
        var at = cur
        for _ in 0..<tracks.count {
            guard let n = pickNext(from: at) else { return nil }
            let spec = tracks[n].spec
            if (try? openSource(spec.url)) != nil {
                fed.append(n)
                feedCursor = n
                return spec
            }
            onMessage?("Skipped \(tracks[n].name)")
            at = n
        }
        return nil
    }

    private func pickNext(from cur: Int) -> Int? {
        if shuffle && repeatMode != .one && tracks.count > 1 {
            var r = Int.random(in: 0..<tracks.count)
            if r == cur { r = (r + 1) % tracks.count }
            return r
        }
        return nextIndex(count: tracks.count, cur: cur, repeat: repeatMode)
    }

    // MARK: - Remembered state

    /// Store the heard file and position for this folder.
    func rememberResume() {
        guard settings.rememberPositions, let folder, let row = heard ?? lastHeard,
              tracks.indices.contains(row) else { return }
        settings.folderResume[folder.path] = Settings.Resume(path: tracks[row].url.path, seconds: position)
        if settings.folderResume.count > 10 {
            let keep = Set(settings.folderResume.keys.sorted().suffix(10))
            settings.folderResume = settings.folderResume.filter { keep.contains($0.key) }
        }
        save()
    }

    private func rememberRecentTrack(_ t: Track) {
        var list = settings.recentTracks.filter { $0 != t.url.path }
        list.insert(t.url.path, at: 0)
        settings.recentTracks = Array(list.prefix(settings.historyEntries))
    }
}
