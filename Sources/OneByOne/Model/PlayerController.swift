import Foundation

/// Owns the current folder's track list and drives the engine. Main thread only.
final class PlayerController {
    let engine = Engine()
    var settings: Settings

    private(set) var folder: URL?
    private(set) var tracks: [Track] = []
    /// Row being heard. `nil` when stopped.
    private(set) var heard: Int?
    /// Rows handed to the engine and not yet heard. The head is heard next.
    private var fed: [Int] = []
    /// Row the gapless feeder is on.
    private var feedCursor: Int?

    var repeatMode: Repeat { didSet { settings.repeatMode = repeatMode; save() } }
    var shuffle: Bool { didSet { settings.shuffle = shuffle; save() } }

    /// Any state change the UI should show.
    var onChange: (() -> Void)?
    /// Short status line for the window.
    var onMessage: ((String) -> Void)?

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
    var position: Double { engine.position }
    var duration: Double? { heard.flatMap { tracks.indices.contains($0) ? tracks[$0].duration : nil } }
    var heardTrack: Track? { heard.flatMap { tracks.indices.contains($0) ? tracks[$0] : nil } }

    func save() { settings.save() }

    // MARK: - Folder

    /// Replace the list with `tracks` from `folder`. Playback keeps going if the file is still listed.
    func setFolder(_ url: URL, tracks newTracks: [Track]) {
        folder = url
        tracks = newTracks
        settings.lastFolder = url.path
        save()
        onChange?()
    }

    // MARK: - Transport

    /// Play a row from its start, or from `seconds` into it.
    func play(row: Int, at seconds: Double = 0) {
        guard tracks.indices.contains(row) else { return }
        fed = [row]
        feedCursor = row
        engine.play(tracks[row].spec, at: seconds)
    }

    /// Space: play or pause. Stopped plays the given row.
    func togglePause(selectedRow: Int?) {
        switch engine.state {
        case .playing: engine.pause()
        case .paused: engine.resume()
        case .stopped: play(row: selectedRow ?? heard ?? 0)
        }
        onChange?()
    }

    func stop() {
        engine.stop()
        heard = nil
        fed = []
        feedCursor = nil
        onChange?()
    }

    func next() {
        guard let cur = heard ?? feedCursor, let n = nextIndex(count: tracks.count, cur: cur, repeat: repeatMode) else {
            stop()
            return
        }
        play(row: n)
    }

    func prev() {
        guard let cur = heard ?? feedCursor else { return }
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

    func rewind(by seconds: Double) {
        seek(to: max(0, position + seconds))
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

    // MARK: - Engine callbacks

    /// The engine started hearing the head of its queue.
    private func trackChanged() {
        guard !fed.isEmpty else { return }
        let row = fed.removeFirst()
        heard = row
        rememberResume()
        onChange?()
    }

    private func finished() {
        heard = nil
        fed = []
        feedCursor = nil
        onChange?()
    }

    /// Next row for the gapless feeder. Skips files that will not open.
    private func nextFeedSpec() -> Spec? {
        guard let cur = feedCursor, !tracks.isEmpty else { return nil }
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

    /// Store the heard file and position for this folder.
    func rememberResume() {
        guard let folder, let row = heard, tracks.indices.contains(row) else { return }
        settings.folderResume[folder.path] = Settings.Resume(path: tracks[row].url.path, seconds: position)
        save()
    }
}
