import AVFoundation
import CFFmpeg

/// Containers and codecs AVFoundation does not open: Matroska, WebM, Opus in CAF, and others.
/// Decodes to float stereo through libswresample. Seeks by timestamp.
final class FFmpegSource: PCMSource {
    private var fmt: UnsafeMutablePointer<AVFormatContext>?
    private var codec: UnsafeMutablePointer<AVCodecContext>?
    private var swr: OpaquePointer?
    private var packet: UnsafeMutablePointer<AVPacket>?
    private var frame: UnsafeMutablePointer<AVFrame>?
    private var streamIndex: Int32 = -1
    private var timeBase = AVRational(num: 1, den: 1)
    private var pendingL: [Float] = []
    private var pendingR: [Float] = []
    private var flushing = false
    private var eof = false
    let format: AVAudioFormat
    let duration: Double?

    init(url: URL) throws {
        let path = url.path
        var ctx: UnsafeMutablePointer<AVFormatContext>? = nil
        guard avformat_open_input(&ctx, path, nil, nil) == 0, let ctx else {
            throw SourceError.open("\(url.lastPathComponent): ffmpeg cannot open")
        }
        fmt = ctx
        guard avformat_find_stream_info(ctx, nil) >= 0 else {
            throw SourceError.open("\(url.lastPathComponent): no stream info")
        }
        let idx = av_find_best_stream(ctx, AVMEDIA_TYPE_AUDIO, -1, -1, nil, 0)
        guard idx >= 0, let st = ctx.pointee.streams[Int(idx)] else {
            throw SourceError.open("\(url.lastPathComponent): no audio stream")
        }
        streamIndex = idx
        timeBase = st.pointee.time_base
        let par = st.pointee.codecpar!
        guard let dec = avcodec_find_decoder(par.pointee.codec_id) else {
            throw SourceError.open("\(url.lastPathComponent): no decoder")
        }
        guard let cc = avcodec_alloc_context3(dec) else {
            throw SourceError.open("\(url.lastPathComponent): no codec context")
        }
        codec = cc
        guard avcodec_parameters_to_context(cc, par) >= 0, avcodec_open2(cc, dec, nil) >= 0 else {
            throw SourceError.open("\(url.lastPathComponent): cannot open decoder")
        }
        let rate = Double(cc.pointee.sample_rate > 0 ? cc.pointee.sample_rate : 48_000)
        guard let fmtOut = AVAudioFormat(standardFormatWithSampleRate: rate, channels: 2) else {
            throw SourceError.open("\(url.lastPathComponent): bad format")
        }
        format = fmtOut
        let d = ctx.pointee.duration
        duration = d > 0 ? Double(d) / Double(AV_TIME_BASE) : nil

        var outLayout = AVChannelLayout()
        av_channel_layout_default(&outLayout, 2)
        var inLayout = cc.pointee.ch_layout
        if inLayout.nb_channels == 0 { av_channel_layout_default(&inLayout, 2) }
        let r = swr_alloc_set_opts2(&swr, &outLayout, AV_SAMPLE_FMT_FLTP, Int32(rate),
                                    &inLayout, cc.pointee.sample_fmt, cc.pointee.sample_rate, 0, nil)
        guard r >= 0, swr_init(swr) >= 0 else {
            throw SourceError.open("\(url.lastPathComponent): resampler")
        }
        packet = av_packet_alloc()
        frame = av_frame_alloc()
    }

    deinit {
        swr_free(&swr)
        if frame != nil { var f = frame; av_frame_free(&f) }
        if packet != nil { var p = packet; av_packet_free(&p) }
        if codec != nil { var c = codec; avcodec_free_context(&c) }
        if fmt != nil { var f = fmt; avformat_close_input(&f) }
    }

    func read(frames: AVAudioFrameCount) -> AVAudioPCMBuffer? {
        let want = Int(frames)
        while pendingL.count < want && !eof {
            if !pump() { break }
        }
        let n = min(want, pendingL.count)
        guard n > 0, let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(n)),
              let l = buf.floatChannelData?[0], let r = buf.floatChannelData?[1] else { return nil }
        for i in 0..<n {
            l[i] = pendingL[i]
            r[i] = pendingR[i]
        }
        pendingL.removeFirst(n)
        pendingR.removeFirst(n)
        buf.frameLength = AVAudioFrameCount(n)
        return buf
    }

    /// Decode one step. Returns false when nothing more can be produced.
    private func pump() -> Bool {
        guard let codec, let frame, let packet else { return false }
        let got = avcodec_receive_frame(codec, frame)
        if got == 0 {
            convertFrame(frame)
            av_frame_unref(frame)
            return true
        }
        if got == AVERROR_EOF_VALUE {
            eof = true
            return false
        }
        // EAGAIN: feed more input, or flush once the file ends.
        if flushing { return avcodec_send_packet(codec, nil) >= 0 || true }
        let rd = av_read_frame(fmt, packet)
        if rd < 0 {
            flushing = true
            _ = avcodec_send_packet(codec, nil)
            return true
        }
        if packet.pointee.stream_index == streamIndex {
            _ = avcodec_send_packet(codec, packet)
        }
        av_packet_unref(packet)
        return true
    }

    private func convertFrame(_ f: UnsafeMutablePointer<AVFrame>) {
        guard let swr else { return }
        let n = Int(f.pointee.nb_samples)
        let cap = n + 256
        let outL = UnsafeMutablePointer<Float>.allocate(capacity: cap)
        let outR = UnsafeMutablePointer<Float>.allocate(capacity: cap)
        defer { outL.deallocate(); outR.deallocate() }
        var outs: [UnsafeMutablePointer<UInt8>?] = [
            UnsafeMutableRawPointer(outL).assumingMemoryBound(to: UInt8.self),
            UnsafeMutableRawPointer(outR).assumingMemoryBound(to: UInt8.self),
        ]
        let produced = outs.withUnsafeMutableBufferPointer { ob -> Int32 in
            withUnsafeBytes(of: f.pointee.data) { raw -> Int32 in
                let ins = raw.bindMemory(to: UnsafePointer<UInt8>?.self)
                return swr_convert(swr, ob.baseAddress, Int32(cap), ins.baseAddress, f.pointee.nb_samples)
            }
        }
        guard produced > 0 else { return }
        for i in 0..<Int(produced) {
            pendingL.append(outL[i])
            pendingR.append(outR[i])
        }
    }

    func seek(to seconds: Double) -> Bool {
        guard let fmt, let codec, streamIndex >= 0 else { return false }
        let ts = Int64(seconds * Double(timeBase.den) / Double(timeBase.num))
        guard av_seek_frame(fmt, streamIndex, ts, AVSEEK_FLAG_BACKWARD) >= 0 else { return false }
        avcodec_flush_buffers(codec)
        pendingL.removeAll()
        pendingR.removeAll()
        flushing = false
        eof = false
        return true
    }
}

/// `AVERROR_EOF`: -MKTAG('E','O','F',' ').
private let AVERROR_EOF_VALUE: Int32 = -541_478_725
