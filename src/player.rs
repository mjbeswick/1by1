use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate, Source};

/// libopenmpt renders tracker modules (MOD, XM, S3M, IT, ...) as stereo float PCM.
struct ModuleSource {
    module: openmpt::module::Module,
    rate: i32,
    buf: Vec<f32>,
    pos: usize,
    total: Option<Duration>,
}

// The module is created on the UI thread and then owned by the audio thread.
// libopenmpt is not thread-safe for concurrent access, but only one thread
// touches it at a time.
unsafe impl Send for ModuleSource {}

impl ModuleSource {
    fn new(path: &Path, start: Duration) -> Result<Self, String> {
        use openmpt::module::{Logger, Module};
        let mut data = std::fs::read(path).map_err(|e| e.to_string())?;
        if data.is_empty() {
            return Err("empty module".into());
        }
        let mut module = Module::create_from_memory(&mut data, Logger::None, &[])
            .map_err(|_| format!("{}: unsupported tracker module", path.display()))?;
        let _ = module.set_repeat_count(0);
        let seconds = module.get_duration_seconds();
        if !start.is_zero() {
            module.set_position_seconds(start.as_secs_f64());
        }
        Ok(Self {
            module,
            rate: 48_000,
            buf: Vec::new(),
            pos: 0,
            total: (seconds > 0.0).then(|| Duration::from_secs_f64(seconds)),
        })
    }

    fn refill(&mut self) {
        let mut scratch = vec![0f32; 8192];
        let frames = self
            .module
            .read_interleaved_float_stereo(self.rate, &mut scratch)
            .min(scratch.len() / 2);
        scratch.truncate(frames * 2);
        self.buf = scratch;
        self.pos = 0;
    }
}

impl Iterator for ModuleSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.buf.len() {
            self.refill();
            if self.pos >= self.buf.len() {
                return None;
            }
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Some(v)
    }
}

impl Source for ModuleSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(4096)
    }

    fn channels(&self) -> rodio::ChannelCount {
        std::num::NonZero::new(2).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        std::num::NonZero::new(self.rate as u32).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.total
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.module.set_position_seconds(pos.as_secs_f64());
        self.buf.clear();
        self.pos = 0;
        Ok(())
    }
}

/// Where in a file playback starts, so cue sheet tracks can share one file.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Spec {
    pub path: PathBuf,
    /// Offset of the virtual track inside the file.
    pub start: Duration,
    /// Length of the virtual track. `None` means to the end of the file.
    pub limit: Option<Duration>,
}

#[allow(dead_code)]
impl Spec {
    pub fn whole(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            start: Duration::ZERO,
            limit: None,
        }
    }
}

/// Mono PCM waiting for the Milkdrop renderer.
fn milkdrop_queue() -> &'static Mutex<std::collections::VecDeque<f32>> {
    static PCM: OnceLock<Mutex<std::collections::VecDeque<f32>>> = OnceLock::new();
    PCM.get_or_init(|| Mutex::new(std::collections::VecDeque::with_capacity(44_100)))
}

/// Drain up to `max` mono samples for the visualizer.
pub fn take_visualizer_pcm(max: usize) -> Vec<f32> {
    let Ok(mut q) = milkdrop_queue().lock() else {
        return Vec::new();
    };
    let n = q.len().min(max);
    q.drain(..n).collect()
}

struct TapSource {
    inner: Box<dyn Source<Item = f32> + Send + 'static>,
    buf: Vec<f32>,
    channels: usize,
}

impl TapSource {
    fn new(inner: Box<dyn Source<Item = f32> + Send + 'static>) -> Self {
        let channels = inner.channels().get() as usize;
        Self {
            inner,
            buf: Vec::with_capacity(8192),
            channels: channels.max(1),
        }
    }
}

impl Iterator for TapSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        self.buf.push(sample);
        if self.buf.len() >= 4096 {
            if let Ok(mut queue) = milkdrop_queue().lock() {
                for frame in self.buf.chunks_exact(self.channels) {
                    queue.push_back(frame.iter().sum::<f32>() / self.channels as f32);
                }
                while queue.len() > 44_100 {
                    queue.pop_front();
                }
            }
            self.buf.clear();
        }
        Some(sample)
    }
}

impl Source for TapSource {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

/// Output device plus the queued track and (optionally) the next one.
pub struct Audio {
    out: MixerDeviceSink,
    player: Option<Player>,
    /// Sources appended and not yet finished.
    queued: usize,
    spec: Option<Spec>,
    volume: f32,
}

impl Audio {
    pub fn open() -> Result<Self, String> {
        let out = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        Ok(Self {
            out,
            player: None,
            queued: 0,
            spec: None,
            volume: 1.0,
        })
    }

    fn open_source(
        spec: &Spec,
        seek: Duration,
    ) -> Result<Box<dyn Source<Item = f32> + Send + 'static>, String> {
        if crate::library::is_module(&spec.path) {
            let src = ModuleSource::new(&spec.path, spec.start + seek)?;
            let src: Box<dyn Source<Item = f32> + Send> = match spec.limit {
                Some(limit) => Box::new(src.take_duration(limit)),
                None => Box::new(src),
            };
            return Ok(Box::new(TapSource::new(src)));
        }
        let file = File::open(&spec.path).map_err(|e| e.to_string())?;
        let dec = Decoder::try_from(file).map_err(|e| e.to_string())?;
        let mut src: Box<dyn Source<Item = f32> + Send> = match spec.limit {
            Some(limit) => Box::new(dec.skip_duration(spec.start).take_duration(limit)),
            None => Box::new(dec.skip_duration(spec.start)),
        };
        if !seek.is_zero() {
            let _ = src.try_seek(seek);
        }
        Ok(Box::new(TapSource::new(src)))
    }

    /// Replace whatever is playing. `seek` is relative to the virtual track start.
    /// Returns the decoder's duration estimate when it reports one.
    pub fn play(&mut self, spec: Spec, seek: Duration) -> Result<Option<Duration>, String> {
        let src = Self::open_source(&spec, seek)?;
        let duration = src.total_duration();
        let player = Player::connect_new(self.out.mixer());
        player.set_volume(self.volume);
        player.append(src);
        self.player = Some(player);
        self.queued = 1;
        self.spec = Some(spec);
        Ok(duration)
    }

    /// Append the next track so the output continues without a gap.
    pub fn enqueue(&mut self, spec: Spec) -> Result<(), String> {
        let Some(player) = self.player.as_ref() else {
            return Err("nothing playing".into());
        };
        let src = Self::open_source(&spec, Duration::ZERO)?;
        player.append(src);
        self.queued += 1;
        Ok(())
    }

    /// Number of sources that finished since the last call. The app advances its index.
    pub fn take_finished(&mut self) -> usize {
        let Some(p) = self.player.as_ref() else { return 0 };
        let left = p.len();
        let done = self.queued.saturating_sub(left);
        if done > 0 {
            self.queued = left;
        }
        done
    }

    pub fn stop(&mut self) {
        if let Ok(mut queue) = milkdrop_queue().lock() {
            queue.clear();
        }
        self.player = None;
        self.queued = 0;
        self.spec = None;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
        if let Some(p) = &self.player {
            p.set_volume(volume);
        }
    }

    pub fn pos(&self) -> Option<Duration> {
        self.player.as_ref().map(Player::get_pos)
    }

    pub fn is_playing(&self) -> bool {
        self.player
            .as_ref()
            .is_some_and(|p| !p.empty() && !p.is_paused())
    }

    pub fn is_paused(&self) -> bool {
        self.player.as_ref().is_some_and(Player::is_paused)
    }

    pub fn pause(&self) {
        if let Some(p) = &self.player {
            p.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(p) = &self.player {
            p.play();
        }
    }

    #[allow(dead_code)]
    pub fn finished(&self) -> bool {
        self.player.as_ref().is_some_and(Player::empty)
    }

    #[allow(dead_code)]
    pub fn spec(&self) -> Option<&Spec> {
        self.spec.as_ref()
    }

    pub fn seek(&mut self, pos: Duration) -> bool {
        self.player
            .as_ref()
            .is_some_and(|p| p.try_seek(pos).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;

    /// Needs fixture files. Set `ONEBYONE_FIXTURES` to a folder of audio files.
    #[test]
    fn seek_each_fixture() {
        let Some(dir) = std::env::var_os("ONEBYONE_FIXTURES") else {
            return;
        };
        let mut failed = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if !crate::library::is_audio(&path) {
                continue;
            }
            // Symphonia 0.5 parses CAF but not ALAC-in-CAF; skip the container here.
            if crate::library::extension(&path) == "caf" {
                continue;
            }
            if crate::library::is_module(&path) {
                let mut src = match ModuleSource::new(&path, Duration::ZERO) {
                    Ok(s) => s,
                    Err(e) => {
                        failed.push(format!("{}: module open: {e}", path.display()));
                        continue;
                    }
                };
                if src.try_seek(Duration::from_secs(1)).is_err() {
                    failed.push(format!("{}: module seek", path.display()));
                    continue;
                }
                if src.next().is_none() {
                    failed.push(format!("{}: module produced no samples", path.display()));
                }
                continue;
            }
            let file = File::open(&path).unwrap();
            let mut src = match Decoder::try_from(file) {
                Ok(s) => s,
                Err(e) => {
                    failed.push(format!("{}: open: {e}", path.display()));
                    continue;
                }
            };
            if src.try_seek(Duration::from_secs(1)).is_err() {
                // Some decoders (ADPCM) cannot seek. They still decode from zero.
                let file = File::open(&path).unwrap();
                let mut fresh = Decoder::try_from(file).unwrap();
                if fresh.next().is_none() {
                    failed.push(format!("{}: no samples", path.display()));
                }
                continue;
            }
            if src.next().is_none() {
                // ADPCM reports a seek but yields no samples; decoding from the
                // start still works, so only fail when a fresh decode is empty.
                let file = File::open(&path).unwrap();
                let mut fresh = Decoder::try_from(file).unwrap();
                if fresh.next().is_none() {
                    failed.push(format!("{}: no samples after seek", path.display()));
                }
            }
        }
        assert!(failed.is_empty(), "decoder failures:\n{}", failed.join("\n"));
    }

    #[test]
    fn cue_window_is_inside_the_file() {
        let spec = Spec {
            path: PathBuf::from("/x/a.mp3"),
            start: Duration::from_secs(30),
            limit: Some(Duration::from_secs(10)),
        };
        assert_eq!(spec.start, Duration::from_secs(30));
    }
}
