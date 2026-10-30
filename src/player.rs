use std::{fs::File, path::Path, time::Duration};

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

/// Output device plus at most one live track. Dropping `Player` stops it.
pub struct Audio {
    out: MixerDeviceSink,
    player: Option<Player>,
}

impl Audio {
    pub fn open() -> Result<Self, String> {
        let out = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        Ok(Self { out, player: None })
    }

    /// Replaces any playing track. `seek` applies after the source starts.
    pub fn play(&mut self, path: &Path, seek: Duration, volume: f32) -> Result<(), String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let src = Decoder::try_from(file).map_err(|e| e.to_string())?;
        let player = Player::connect_new(self.out.mixer());
        player.set_volume(volume);
        player.append(src);
        if !seek.is_zero() {
            // ponytail: seek failure leaves the track at 0. No retry.
            let _ = player.try_seek(seek);
        }
        self.player = Some(player);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.player = None;
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some(p) = &self.player {
            p.set_volume(volume);
        }
    }

    pub fn pos(&self) -> Option<Duration> {
        self.player.as_ref().map(|p| p.get_pos())
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.empty())
    }

    /// True once a track has played to its end. Stopped state is `None`, not this.
    pub fn finished(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;

    /// Needs fixture files. Set `ONEBYONE_FIXTURES` to a folder of audio files.
    /// Each file must accept a 1 s seek and still yield samples.
    #[test]
    fn seek_each_fixture() {
        let Some(dir) = std::env::var_os("ONEBYONE_FIXTURES") else {
            return;
        };
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if !crate::library::is_audio(&path) {
                continue;
            }
            let file = File::open(&path).unwrap();
            let mut src = Decoder::try_from(file)
                .unwrap_or_else(|e| panic!("{}: open failed: {e}", path.display()));
            src.try_seek(Duration::from_secs(1))
                .unwrap_or_else(|e| panic!("{}: seek failed: {e}", path.display()));
            assert!(src.next().is_some(), "{}: no samples after seek", path.display());
        }
    }
}
