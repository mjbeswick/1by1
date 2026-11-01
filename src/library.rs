#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use lofty::{
    file::TaggedFileExt,
    prelude::{Accessor, AudioFile, ItemKey},
    probe::Probe,
    tag::Tag,
};

use crate::logic::{natural_cmp, size_text};

/// Every container extension the bundled decoder stack can open.
/// Symphonia formats: MPEG audio, AAC/ADTS, ISO MP4, FLAC, Ogg, WAV, AIFF, CAF, Matroska.
/// Tracker modules handled by libopenmpt.
pub const MODULE_EXTS: [&str; 26] = [
    "mod", "xm", "s3m", "it", "mtm", "umx", "mo3", "mptm", "669", "far", "med", "stm",
    "ptm", "ult", "okt", "mdz", "dbm", "digi", "dmf", "dsm", "amf", "ams", "gdm", "j2b",
    "mt2", "psm",
];

pub const AUDIO_EXTS: [&str; 25] = [
    "mp1", "mp2", "mp3", "mpa", "mpga", "aac", "adts", "m4a", "m4b", "m4p", "m4r", "mp4",
    "flac", "ogg", "oga", "wav", "wave", "bwf", "aif", "aiff", "aifc", "caf", "mkv", "mka",
    "webm",
];

#[derive(Clone, Debug, Default)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<String>,
    pub track: Option<String>,
    pub comment: Option<String>,
}

impl Tags {
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.artist.is_none() && self.album.is_none()
    }

    /// `Artist - Title`, else title, else artist.
    pub fn artist_title(&self) -> Option<String> {
        match (&self.artist, &self.title) {
            (Some(a), Some(t)) if !a.is_empty() && !t.is_empty() && a != t => {
                Some(format!("{a} - {t}"))
            }
            (_, Some(t)) if !t.is_empty() => Some(t.clone()),
            (Some(a), _) if !a.is_empty() => Some(a.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AudioInfo {
    pub duration: Option<std::time::Duration>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bits: Option<u8>,
    pub tags: Tags,
    pub decoder: &'static str,
    /// Tracker or container description, e.g. "Impulse Tracker (IT)".
    pub format: Option<String>,
}

impl AudioInfo {
    /// `160 kbps, 44.1 kHz, Stereo` style summary.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(b) = self.bitrate {
            parts.push(format!("{b} kbps"));
        }
        if let Some(s) = self.sample_rate {
            parts.push(format!("{:.1} kHz", s as f32 / 1000.0));
        }
        if let Some(c) = self.channels {
            parts.push(channels_text(c).to_string());
        }
        parts.join(", ")
    }

    pub fn decoder(&self) -> &'static str {
        self.decoder
    }
}

pub fn channels_text(c: u8) -> &'static str {
    match c {
        1 => "Mono",
        2 => "Stereo",
        _ => "Multichannel",
    }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub path: PathBuf,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub info: Option<AudioInfo>,
    /// Cue sheet virtual track: start offset and end offset inside `path`.
    pub cue: Option<(std::time::Duration, Option<std::time::Duration>)>,
}

impl Track {
    pub fn new(path: PathBuf) -> Self {
        let meta = fs::metadata(&path).ok();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = extension(&path);
        Self {
            path,
            name,
            ext,
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: meta.as_ref().and_then(|m| m.modified().ok()),
            created: meta.as_ref().and_then(|m| m.created().ok()),
            info: None,
            cue: None,
        }
    }

    pub fn is_cue(&self) -> bool {
        self.cue.is_some()
    }

    /// Title for the window and lists: tag title when available, else file name.
    pub fn display(&self) -> &str {
        self.info
            .as_ref()
            .and_then(|i| i.tags.title.as_deref())
            .filter(|t| !t.is_empty())
            .unwrap_or(&self.name)
    }

    pub fn artist_title(&self) -> String {
        self.info
            .as_ref()
            .and_then(|i| i.tags.artist_title())
            .unwrap_or_else(|| self.name.clone())
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        if let Some((start, end)) = self.cue {
            return end.map(|e| e.saturating_sub(start)).or_else(|| {
                self.info
                    .as_ref()
                    .and_then(|i| i.duration)
                    .map(|d| d.saturating_sub(start))
            });
        }
        self.info.as_ref().and_then(|i| i.duration)
    }

    pub fn size_text(&self) -> String {
        size_text(self.size)
    }

    /// Short type text: upper-case extension, `CUE` for cue tracks.
    pub fn type_text(&self) -> String {
        if self.is_cue() {
            "CUE".into()
        } else {
            self.ext.to_uppercase()
        }
    }

    /// Extra info column text.
    pub fn info_text(&self) -> String {
        match &self.info {
            Some(i) if !i.tags.is_empty() => i
                .tags
                .artist_title()
                .unwrap_or_else(|| i.summary()),
            Some(i) => i.summary(),
            None => String::new(),
        }
    }
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub fn is_module(path: &Path) -> bool {
    MODULE_EXTS.contains(&extension(path).as_str())
}

pub fn is_audio(path: &Path) -> bool {
    let ext = extension(path);
    AUDIO_EXTS.contains(&ext.as_str()) || MODULE_EXTS.contains(&ext.as_str())
}

pub fn is_playlist(path: &Path) -> bool {
    matches!(extension(path).as_str(), "m3u" | "m3u8")
}

pub fn is_cue(path: &Path) -> bool {
    extension(path) == "cue"
}

/// Files directly in `dir`, sorted by name. Hidden entries are skipped.
pub fn audio_files(dir: &Path) -> Vec<Track> {
    list(dir, |t| t.is_file(), is_audio).into_iter().map(Track::new).collect()
}

/// Audio files, cue sheets expanded into virtual tracks, and playlist entries.
pub fn folder_tracks(dir: &Path) -> Vec<Track> {
    let mut out = Vec::new();
    for path in list(dir, |t| t.is_file(), |p| is_audio(p) || is_cue(p) || is_playlist(p)) {
        if is_cue(&path) {
            let mut cues = cue_tracks(&path);
            out.append(&mut cues);
        } else if is_playlist(&path) {
            out.push(Track::new(path));
        } else {
            out.push(Track::new(path));
        }
    }
    out
}

/// Subfolders directly in `dir`, sorted by name. Symlinks are not followed.
pub fn subdirs(dir: &Path) -> Vec<PathBuf> {
    list(dir, |t| t.is_dir(), |_| true)
}

pub fn child_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    subdirs(dir)
        .into_iter()
        .map(|p| {
            let n = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            (n, p)
        })
        .collect()
}

fn list(dir: &Path, kind: fn(fs::FileType) -> bool, keep: fn(&Path) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter(|e| e.file_type().map(kind).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| keep(p))
        .collect();
    out.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    out
}

/// Read tags and audio parameters. Cheap enough for a few hundred files.
pub fn scan_info(path: &Path) -> Option<AudioInfo> {
    if is_module(path) {
        return scan_module(path);
    }
    let tagged = Probe::open(path).ok()?.read().ok()?;
    let props = tagged.properties();
    let duration = props.duration();
    let mut info = AudioInfo {
        duration: (!duration.is_zero()).then_some(duration),
        bitrate: props.audio_bitrate(),
        sample_rate: props.sample_rate(),
        channels: props.channels(),
        bits: props.bit_depth(),
        decoder: decoder_name(path),
        ..Default::default()
    };
    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        info.tags = read_tags(tag);
    }
    Some(info)
}

fn read_tags(tag: &Tag) -> Tags {
    let comment = tag
        .get_string(ItemKey::Comment)
        .or_else(|| tag.get_string(ItemKey::Description))
        .map(str::to_string);
    Tags {
        title: tag.title().map(|s| s.to_string()),
        artist: tag.artist().map(|s| s.to_string()),
        album: tag.album().map(|s| s.to_string()),
        year: tag.get_string(ItemKey::Year).map(|y| y.to_string()),
        track: tag.track().map(|t| t.to_string()),
        comment,
    }
}

/// Read module metadata and duration through libopenmpt.
pub fn scan_module(path: &Path) -> Option<AudioInfo> {
    use openmpt::module::metadata::MetadataKey;
    use openmpt::module::{Logger, Module};

    let mut data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }
    let mut m = Module::create_from_memory(&mut data, Logger::None, &[]).ok()?;
    let seconds = m.get_duration_seconds();
    let title = m.get_metadata(MetadataKey::ModuleTitle).filter(|s| !s.is_empty());
    let artist = m.get_metadata(MetadataKey::ModuleArtist).filter(|s| !s.is_empty());
    let tracker = m.get_metadata(MetadataKey::ModuleTracker).filter(|s| !s.is_empty());
    let kind = m.get_metadata(MetadataKey::TypeName).filter(|s| !s.is_empty());
    let ext = m.get_metadata(MetadataKey::TypeExt).unwrap_or_else(|| extension(path));
    let date = m.get_metadata(MetadataKey::ModuleSaveDate).filter(|s| !s.is_empty());
    let message = m.get_metadata(MetadataKey::SongMessage).filter(|s| !s.is_empty());
    let format = match (kind, tracker) {
        (Some(k), Some(t)) if k != t => Some(format!("{k}, saved by {t} ({})", ext.to_uppercase())),
        (Some(k), _) => Some(format!("{k} ({})", ext.to_uppercase())),
        (None, Some(t)) => Some(format!("{t} ({})", ext.to_uppercase())),
        (None, None) => Some(format!("Tracker module ({})", ext.to_uppercase())),
    };
    Some(AudioInfo {
        duration: (seconds > 0.0).then(|| std::time::Duration::from_secs_f64(seconds)),
        bitrate: None,
        sample_rate: Some(48_000),
        channels: Some(2),
        bits: Some(16),
        tags: Tags {
            title,
            artist,
            album: None,
            year: date.map(|d| d.chars().take(4).collect()),
            track: None,
            comment: message.map(|m| m.replace('\r', "")),
        },
        decoder: "Tracker",
        format,
    })
}

pub fn decoder_name(path: &Path) -> &'static str {
    if is_module(path) {
        return "Tracker";
    }
    match extension(path).as_str() {
        "mp1" | "mp2" | "mp3" | "mpa" | "mpga" => "MPEG",
        "flac" => "FLAC",
        "wav" | "wave" | "bwf" | "aif" | "aiff" | "aifc" => "PCM",
        "ogg" | "oga" => "Vorbis",
        "aac" | "adts" | "m4a" | "m4b" | "m4p" | "m4r" | "mp4" => "AAC",
        "caf" => "CAF",
        "mkv" | "mka" | "webm" => "Matroska",
        _ => "Audio",
    }
}

/// Parse an m3u/m3u8 playlist. Relative entries resolve against the list's folder.
pub fn read_playlist(path: &Path) -> Vec<PathBuf> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let base = path.parent().unwrap_or(Path::new("."));
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let p = PathBuf::from(l);
            if p.is_absolute() { p } else { base.join(p) }
        })
        .filter(|p| p.is_file())
        .collect()
}

/// One `TRACK` entry of a cue sheet: offset in the file plus tags.
#[derive(Clone, Debug)]
pub struct CueTrack {
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start: std::time::Duration,
}

/// Parse `INDEX 01 mm:ss:ff` lines. The file the cue sheet points at is `audio`.
pub fn read_cue(path: &Path) -> (Option<PathBuf>, Vec<CueTrack>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (None, Vec::new());
    };
    let base = path.parent().unwrap_or(Path::new("."));
    let mut audio = None;
    let mut out: Vec<CueTrack> = Vec::new();
    let (mut title, mut performer) = (None, None);
    for line in text.lines() {
        let mut it = line.trim().splitn(2, char::is_whitespace);
        let key = it.next().unwrap_or("").to_ascii_uppercase();
        let rest = it.next().unwrap_or("").trim().trim_matches('"');
        match key.as_str() {
            "FILE" => {
                let name = rest.trim_end_matches(" WAVE").trim_end_matches(" MP3");
                let p = PathBuf::from(name);
                audio = Some(if p.is_absolute() { p } else { base.join(p) });
            }
            "TRACK" => {
                out.push(CueTrack {
                    title: title.take(),
                    performer: performer.take(),
                    start: std::time::Duration::ZERO,
                });
            }
            "TITLE" => title = Some(rest.to_string()),
            "PERFORMER" => performer = Some(rest.to_string()),
            "INDEX" => {
                if let Some(last) = out.last_mut() {
                    let mut n = rest.split_whitespace();
                    let idx = n.next().unwrap_or("");
                    let stamp = n.next().unwrap_or("");
                    if idx == "01" {
                        if let Some(d) = parse_cue_time(stamp) {
                            last.start = d;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // END offsets come from the next track, plus the file duration for the last one.
    (audio, out)
}

fn parse_cue_time(s: &str) -> Option<std::time::Duration> {
    let mut it = s.split(':');
    let m: u64 = it.next()?.parse().ok()?;
    let sec: u64 = it.next()?.parse().ok()?;
    let frames: u64 = it.next().unwrap_or("0").parse().ok()?;
    Some(std::time::Duration::from_millis(
        m * 60_000 + sec * 1_000 + frames * 1000 / 75,
    ))
}

/// Expand a cue sheet into virtual tracks. Needs the audio length for the last entry.
pub fn cue_tracks(path: &Path) -> Vec<Track> {
    let (audio, entries) = read_cue(path);
    let Some(audio) = audio else { return Vec::new() };
    let total = scan_info(&audio).and_then(|i| i.duration);
    let mut base = Track::new(audio);
    base.cue = Some((std::time::Duration::ZERO, total));
    let mut out = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let end = entries.get(i + 1).map(|n| n.start).or(total);
        let mut t = base.clone();
        t.cue = Some((e.start, end));
        t.name = e
            .title
            .clone()
            .unwrap_or_else(|| format!("{} - track {:02}", base.name, i + 1));
        t.info = Some(AudioInfo {
            duration: end.map(|e2| e2.saturating_sub(e.start)),
            tags: Tags {
                title: e.title.clone(),
                artist: e.performer.clone(),
                ..Default::default()
            },
            decoder: "CUE",
            ..Default::default()
        });
        out.push(t);
    }
    out
}

/// Write the editable tag fields back to the file.
pub fn write_tags(path: &Path, tags: &Tags) -> Result<(), String> {
    if is_module(path) {
        return Err("tracker modules do not store editable tags".into());
    }
    use lofty::{
        config::WriteOptions,
        prelude::AudioFile,
        tag::{ItemKey, Tag, TagType},
    };

    let mut tagged = Probe::open(path)
        .map_err(|e| e.to_string())?
        .read()
        .map_err(|e| e.to_string())?;
    if tagged.primary_tag_mut().is_none() {
        let tt = match extension(path).as_str() {
            "flac" | "ogg" | "oga" | "opus" => TagType::VorbisComments,
            "m4a" | "m4b" | "mp4" | "m4p" => TagType::Mp4Ilst,
            "wav" => TagType::RiffInfo,
            "aif" | "aiff" | "aifc" => TagType::AiffText,
            _ => TagType::Id3v2,
        };
        tagged.insert_tag(Tag::new(tt));
    }
    let tag = tagged.primary_tag_mut().ok_or("no writable tag")?;
    let fields = [
        (ItemKey::TrackTitle, tags.title.clone()),
        (ItemKey::TrackArtist, tags.artist.clone()),
        (ItemKey::AlbumTitle, tags.album.clone()),
        (ItemKey::Year, tags.year.clone()),
        (ItemKey::TrackNumber, tags.track.clone()),
        (ItemKey::Comment, tags.comment.clone()),
    ];
    for (key, value) in fields {
        match value {
            Some(v) if !v.trim().is_empty() => {
                tag.insert_text(key, v);
            }
            _ => tag.remove_key(key),
        }
    }
    tagged
        .save_to_path(path, WriteOptions::default())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_write_roundtrip() {
        let Some(dir) = std::env::var_os("ONEBYONE_FIXTURES") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let Some(src) = std::fs::read_dir(&dir).ok().and_then(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .find(|p| extension(p) == "mp3")
        }) else {
            return;
        };
        let tmp = std::env::temp_dir().join("onebyone-tag-test.mp3");
        std::fs::copy(&src, &tmp).unwrap();
        let tags = Tags {
            title: Some("Roundtrip".into()),
            artist: Some("Tester".into()),
            album: Some("Album".into()),
            year: Some("2026".into()),
            track: Some("3".into()),
            comment: Some("note".into()),
        };
        write_tags(&tmp, &tags).unwrap();
        let info = scan_info(&tmp).unwrap();
        assert_eq!(info.tags.title.as_deref(), Some("Roundtrip"));
        assert_eq!(info.tags.artist.as_deref(), Some("Tester"));
        assert_eq!(info.tags.track.as_deref(), Some("3"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn module_metadata() {
        // Minimal 4-channel ProTracker module with an empty pattern.
        let mut data = Vec::new();
        let mut title = b"Test Module".to_vec();
        title.resize(20, 0);
        data.extend_from_slice(&title);
        for _ in 0..31 {
            data.extend_from_slice(&[0u8; 30]);
        }
        data.push(1); // song length
        data.push(0x7f); // restart
        let mut order = [0u8; 128];
        order[0] = 0;
        data.extend_from_slice(&order);
        data.extend_from_slice(b"M.K.");
        data.extend_from_slice(&[0u8; 1024]); // one empty pattern
        let path = std::env::temp_dir().join("oneby1-module-test.mod");
        std::fs::write(&path, &data).unwrap();
        let info = scan_info(&path).expect("module metadata");
        assert_eq!(info.tags.title.as_deref(), Some("Test Module"));
        assert_eq!(info.decoder, "Tracker");
        assert!(info.duration.unwrap_or_default().as_secs() > 1);
        assert!(info.format.is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cue_time() {
        let d = parse_cue_time("01:23:45").unwrap();
        assert_eq!(d.as_millis(), 83_000 + 600);
        assert!(is_audio(Path::new("/x/a.MP3")));
        assert!(!is_audio(Path::new("/x/a.txt")));
        assert!(is_playlist(Path::new("/x/a.m3u8")));
    }
}
