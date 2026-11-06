import AVFoundation
import COpenMPT
import CVorbis

/// One playable item. A cue sheet can point several items at one file.
struct Spec: Equatable {
    var url: URL
    /// Offset of the virtual track inside the file, in seconds.
    var start: Double = 0
    /// Length of the virtual track. `nil` means to the end of the file.
    var limit: Double? = nil

    static func whole(_ url: URL) -> Spec { Spec(url: url) }
}

/// Module extensions handled by libopenmpt. Same list the Rust build used.
let moduleExtensions: Set<String> = [
    "mod", "xm", "s3m", "it", "mtm", "umx", "mo3", "mptm", "669", "far", "med", "stm",
    "ptm", "ult", "okt", "mdz", "dbm", "dmf", "dsm", "amf", "gdm", "j2b", "mt2", "psm",
]

/// Decoded PCM in its own format. Converted to the output format by `Engine`.
protocol PCMSource: AnyObject {
    var format: AVAudioFormat { get }
    /// Length of the whole source in seconds, if known.
    var duration: Double? { get }
    /// Next block, or `nil` at end of stream.
    func read(frames: AVAudioFrameCount) -> AVAudioPCMBuffer?
    /// Seek to an absolute position in seconds. `false` when the decoder cannot.
    func seek(to seconds: Double) -> Bool
}

enum SourceError: Error, CustomStringConvertible {
    case open(String)
    var description: String {
        switch self {
        case .open(let s): return s
        }
    }
}

/// Picks a decoder by extension. Ogg tries Vorbis first, then AVFoundation (Opus in Ogg).
func openSource(_ url: URL) throws -> PCMSource {
    let ext = url.pathExtension.lowercased()
    if moduleExtensions.contains(ext) {
        return try ModuleSource(url: url)
    }
    if ext == "ogg" || ext == "oga" {
        if let v = try? VorbisSource(url: url) { return v }
    }
    return try AVFileSource(url: url)
}

// MARK: - AVFoundation

final class AVFileSource: PCMSource {
    private let file: AVAudioFile
    let format: AVAudioFormat

    init(url: URL) throws {
        do {
            file = try AVAudioFile(forReading: url)
        } catch {
            throw SourceError.open("\(url.lastPathComponent): \(error.localizedDescription)")
        }
        format = file.processingFormat
    }

    var duration: Double? {
        Double(file.length) / format.sampleRate
    }

    func read(frames: AVAudioFrameCount) -> AVAudioPCMBuffer? {
        guard let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frames) else { return nil }
        do {
            try file.read(into: buf, frameCount: frames)
        } catch {
            return nil
        }
        return buf.frameLength > 0 ? buf : nil
    }

    func seek(to seconds: Double) -> Bool {
        let frame = AVAudioFramePosition(seconds * format.sampleRate)
        guard frame >= 0 && frame <= file.length else { return false }
        file.framePosition = frame
        return true
    }
}

// MARK: - Ogg Vorbis (libvorbisfile)

final class VorbisSource: PCMSource {
    private let vf: UnsafeMutablePointer<OggVorbis_File>
    let format: AVAudioFormat
    private let total: Double?

    init(url: URL) throws {
        vf = .allocate(capacity: 1)
        guard ov_fopen(url.path, vf) == 0 else {
            vf.deallocate()
            throw SourceError.open("\(url.lastPathComponent): not Ogg Vorbis")
        }
        guard let info = ov_info(vf, -1) else {
            ov_clear(vf)
            vf.deallocate()
            throw SourceError.open("\(url.lastPathComponent): no stream info")
        }
        let rate = Double(info.pointee.rate)
        let channels = AVAudioChannelCount(max(1, info.pointee.channels))
        guard let fmt = AVAudioFormat(standardFormatWithSampleRate: rate, channels: channels) else {
            ov_clear(vf)
            vf.deallocate()
            throw SourceError.open("\(url.lastPathComponent): bad format")
        }
        format = fmt
        let t = ov_time_total(vf, -1)
        total = t > 0 ? t : nil
    }

    deinit {
        ov_clear(vf)
        vf.deallocate()
    }

    var duration: Double? { total }

    func read(frames: AVAudioFrameCount) -> AVAudioPCMBuffer? {
        guard let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frames) else { return nil }
        var pcm: UnsafeMutablePointer<UnsafeMutablePointer<Float>?>? = nil
        var bitstream: Int32 = 0
        let got = ov_read_float(vf, &pcm, Int32(frames), &bitstream)
        guard got > 0, let pcm else { return nil }
        let n = Int(got)
        for c in 0..<Int(format.channelCount) {
            guard let src = pcm[c], let dst = buf.floatChannelData?[c] else { continue }
            dst.update(from: src, count: n)
        }
        buf.frameLength = AVAudioFrameCount(n)
        return buf
    }

    func seek(to seconds: Double) -> Bool {
        ov_time_seek(vf, seconds) == 0
    }
}

// MARK: - Tracker modules (libopenmpt)

final class ModuleSource: PCMSource {
    private let mod: OpaquePointer
    /// Output rate for libopenmpt. Converted later like any other source.
    static let rate: Int32 = 48_000
    let format: AVAudioFormat
    private let total: Double?
    private var scratch: [Float]

    init(url: URL) throws {
        let data: Data
        do {
            data = try Data(contentsOf: url)
        } catch {
            throw SourceError.open("\(url.lastPathComponent): \(error.localizedDescription)")
        }
        guard !data.isEmpty else { throw SourceError.open("\(url.lastPathComponent): empty") }
        let made: OpaquePointer? = data.withUnsafeBytes { raw in
            openmpt_module_create_from_memory2(
                raw.baseAddress, raw.count, nil, nil, nil, nil, nil, nil, nil)
        }
        guard let mod = made else {
            throw SourceError.open("\(url.lastPathComponent): unsupported tracker module")
        }
        self.mod = mod
        _ = openmpt_module_set_repeat_count(mod, 0)
        let secs = openmpt_module_get_duration_seconds(mod)
        total = secs > 0 ? secs : nil
        format = AVAudioFormat(standardFormatWithSampleRate: Double(Self.rate), channels: 2)!
        scratch = [Float](repeating: 0, count: 4096 * 2)
    }

    deinit { openmpt_module_destroy(mod) }

    var duration: Double? { total }

    func read(frames: AVAudioFrameCount) -> AVAudioPCMBuffer? {
        let want = Int(frames)
        if scratch.count < want * 2 { scratch = [Float](repeating: 0, count: want * 2) }
        let got = scratch.withUnsafeMutableBufferPointer { p in
            openmpt_module_read_interleaved_float_stereo(mod, Self.rate, want, p.baseAddress)
        }
        guard got > 0, let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(got)) else {
            return nil
        }
        let n = Int(got)
        let l = buf.floatChannelData![0], r = buf.floatChannelData![1]
        for i in 0..<n {
            l[i] = scratch[i * 2]
            r[i] = scratch[i * 2 + 1]
        }
        buf.frameLength = AVAudioFrameCount(n)
        return buf
    }

    func seek(to seconds: Double) -> Bool {
        openmpt_module_set_position_seconds(mod, seconds) >= 0
    }
}
