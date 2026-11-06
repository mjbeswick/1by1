import AVFoundation

/// Gapless playback engine. All calls happen on the main thread.
///
/// Decoding runs in `pump()`, which keeps about one second of PCM scheduled on the
/// player node. When the feeding track ends, `nextProvider` supplies the next spec, so
/// the next track is already in the node when the current one finishes.
/// `onTrackChanged` fires when a track starts being heard, not when it is fed.
final class Engine {
    enum State { case stopped, playing, paused }

    /// Output format every source is converted to. Fixed, so gapless joins never change format.
    static let outputRate: Double = 48_000
    let outFormat = AVAudioFormat(standardFormatWithSampleRate: Engine.outputRate, channels: 2)!

    /// Called with the next spec to feed. The app advances its own cursor here.
    var nextProvider: (() -> Spec?)?
    /// A track became the one being heard.
    var onTrackChanged: ((Spec) -> Void)?
    /// Everything played and nothing left to feed.
    var onFinished: (() -> Void)?
    /// A spec could not be opened.
    var onError: ((Spec, String) -> Void)?

    private(set) var state: State = .stopped
    var volume: Float {
        get { node.volume }
        set { node.volume = newValue }
    }

    private let engine = AVAudioEngine()
    private let node = AVAudioPlayerNode()
    private var tracks: [Track] = []
    private var timer: Timer?
    private var nextTrackID = 0
    private static let lookaheadFrames: Int64 = Int64(Engine.outputRate)

    init() {
        engine.attach(node)
        engine.connect(node, to: engine.mainMixerNode, format: outFormat)
        engine.prepare()
        try? engine.start()
        timer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            self?.pump()
        }
    }

    /// Spec being heard now.
    var current: Spec? { tracks.first?.spec }

    /// Seconds into the virtual track being heard.
    var position: Double {
        guard let t = tracks.first else { return 0 }
        return t.seekBase + Double(t.playedFrames) / Engine.outputRate
    }

    /// Length of the virtual track being heard, if known.
    var duration: Double? { tracks.first?.duration }

    /// Start `spec` from `seconds` into its virtual track. Replaces what plays.
    func play(_ spec: Spec, at seconds: Double = 0) {
        let keepPaused = state == .paused
        reset()
        guard let track = makeTrack(spec, at: seconds) else { return }
        tracks = [track]
        node.play()
        state = .playing
        pump()
        onTrackChanged?(spec)
        if keepPaused { pause() }
    }

    /// Seek inside the current track.
    func seek(to seconds: Double) {
        guard let spec = current else { return }
        play(spec, at: max(0, seconds))
    }

    func pause() {
        guard state == .playing else { return }
        node.pause()
        state = .paused
    }

    func resume() {
        guard state == .paused else { return }
        node.play()
        state = .playing
    }

    func stop() {
        reset()
        state = .stopped
    }

    // MARK: - Internals

    private func reset() {
        for t in tracks { t.dead = true }
        tracks.removeAll()
        node.stop()
    }

    private func makeTrack(_ spec: Spec, at seconds: Double) -> Track? {
        do {
            let src = try openSource(spec.url)
            let offset = spec.start + seconds
            guard offset <= 0 || src.seek(to: offset) else {
                onError?(spec, "\(spec.url.lastPathComponent): cannot seek")
                return nil
            }
            nextTrackID += 1
            return try Track(id: nextTrackID, spec: spec, source: src, seekBase: seconds, outFormat: outFormat)
        } catch {
            onError?(spec, "\(error)")
            return nil
        }
    }

    /// Keep the node fed. Runs on a timer and after every finished buffer.
    private func pump() {
        var guardCount = 0
        while queuedFrames() < Engine.lookaheadFrames && state != .stopped {
            guardCount += 1
            if guardCount > 200 { break }
            if let last = tracks.last, !last.feedDone {
                feed(last)
                continue
            }
            // Current feeder is finished (or none). Ask for the next track.
            guard let spec = nextProvider?() else { break }
            guard let t = makeTrack(spec, at: 0) else { continue }
            tracks.append(t)
        }
    }

    /// Returns true when a buffer was scheduled.
    @discardableResult
    private func feed(_ t: Track) -> Bool {
        guard let out = t.nextBuffer() else {
            if let tail = t.flush(), tail.frameLength > 0 {
                schedule(tail, on: t)
            }
            t.feedDone = true
            return false
        }
        schedule(out, on: t)
        return true
    }

    private func schedule(_ buf: AVAudioPCMBuffer, on t: Track) {
        let n = Int64(buf.frameLength)
        t.fedFrames += n
        node.scheduleBuffer(buf, completionCallbackType: .dataPlayedBack) { [weak self, weak t] _ in
            DispatchQueue.main.async {
                guard let self, let t, !t.dead else { return }
                self.played(t, frames: n)
            }
        }
    }

    private func played(_ t: Track, frames: Int64) {
        t.playedFrames += frames
        // Only the head track counts as heard. It finishes once everything fed has played.
        while let head = tracks.first, head.feedDone, head.playedFrames >= head.fedFrames {
            tracks.removeFirst()
            if let next = tracks.first {
                onTrackChanged?(next.spec)
            }
        }
        if tracks.isEmpty && state == .playing {
            state = .stopped
            onFinished?()
            return
        }
        pump()
    }

    private func queuedFrames() -> Int64 {
        tracks.reduce(0) { $0 + ($1.fedFrames - $1.playedFrames) }
    }
}

/// One source being fed into the player node.
final class Track {
    let id: Int
    let spec: Spec
    let source: PCMSource
    let seekBase: Double
    var feedDone = false
    var dead = false
    var fedFrames: Int64 = 0
    var playedFrames: Int64 = 0

    private let converter: AVAudioConverter?
    private let outFormat: AVAudioFormat
    /// Stop feeding this many seconds into the virtual track, if the spec has a window.
    private let windowSeconds: Double?
    private static let chunk: AVAudioFrameCount = 4096

    init(id: Int, spec: Spec, source: PCMSource, seekBase: Double, outFormat: AVAudioFormat) throws {
        self.id = id
        self.spec = spec
        self.source = source
        self.seekBase = seekBase
        self.outFormat = outFormat
        if source.format.sampleRate == outFormat.sampleRate,
           source.format.channelCount == outFormat.channelCount,
           source.format.commonFormat == outFormat.commonFormat,
           source.format.isInterleaved == outFormat.isInterleaved {
            converter = nil
        } else {
            guard let c = AVAudioConverter(from: source.format, to: outFormat) else {
                throw SourceError.open("\(spec.url.lastPathComponent): no converter")
            }
            converter = c
        }
        windowSeconds = spec.limit.map { max(0, $0 - seekBase) }
    }

    /// Length of what this track plays, in seconds.
    var duration: Double? {
        let whole = source.duration.map { max(0, $0 - spec.start - seekBase) }
        switch (whole, windowSeconds) {
        case let (w?, l?): return min(w, l)
        case let (w?, nil): return w
        case let (nil, l?): return l
        default: return nil
        }
    }

    /// Next block in the output format, or `nil` at end of source or window.
    func nextBuffer() -> AVAudioPCMBuffer? {
        if let w = windowSeconds, Double(fedFrames) / Engine.outputRate >= w { return nil }
        guard let raw = source.read(frames: Track.chunk) else { return nil }
        return convert(raw)
    }

    /// Drain whatever the converter still holds after the source ends.
    func flush() -> AVAudioPCMBuffer? {
        guard let c = converter, let out = AVAudioPCMBuffer(pcmFormat: outFormat, frameCapacity: 4096) else {
            return nil
        }
        var err: NSError?
        let status = c.convert(to: out, error: &err) { _, inStatus in
            inStatus.pointee = .endOfStream
            return nil
        }
        return status == .error ? nil : out
    }

    private func convert(_ input: AVAudioPCMBuffer) -> AVAudioPCMBuffer? {
        guard let c = converter else { return input }
        let ratio = outFormat.sampleRate / input.format.sampleRate
        let cap = AVAudioFrameCount(Double(input.frameLength) * ratio) + 64
        guard let out = AVAudioPCMBuffer(pcmFormat: outFormat, frameCapacity: cap) else { return nil }
        var fed = false
        var err: NSError?
        let status = c.convert(to: out, error: &err) { _, inStatus in
            if fed {
                inStatus.pointee = .noDataNow
                return nil
            }
            fed = true
            inStatus.pointee = .haveData
            return input
        }
        if status == .error { return nil }
        return out
    }
}
