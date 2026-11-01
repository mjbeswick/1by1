#![allow(dead_code)]

use std::{cmp::Ordering, time::Duration};

use serde::{Deserialize, Serialize};

/// Repeat behaviour of the play queue.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    /// Stop at the end of the list.
    #[default]
    Off,
    /// Continue at the top of the list.
    All,
    /// Play the same track again.
    One,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Repeat::Off => "Off",
            Repeat::All => "All",
            Repeat::One => "Track",
        }
    }
}

/// Column used for the current list order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Name,
    Length,
    Size,
    Type,
    Modified,
    Created,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Length => "Length",
            SortKey::Size => "Size",
            SortKey::Type => "Type",
            SortKey::Modified => "Modified",
            SortKey::Created => "Created",
        }
    }
}

/// Index after `cur`. `None` when play stops.
pub fn next_index(len: usize, cur: usize, repeat: Repeat) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match repeat {
        Repeat::One => Some(cur),
        _ if cur + 1 < len => Some(cur + 1),
        Repeat::All => Some(0),
        Repeat::Off => None,
    }
}

/// Index before `cur`. Wraps to the end with repeat-all.
pub fn prev_index(len: usize, cur: usize, repeat: Repeat) -> usize {
    if len == 0 {
        return 0;
    }
    if cur == 0 {
        return if repeat == Repeat::All { len - 1 } else { 0 };
    }
    cur - 1
}

/// Walk forward from `cur` until `try_play` accepts one. Gives up after one full pass,
/// so an all-bad folder with repeat-all ends instead of spinning.
pub fn advance_playable(
    len: usize,
    cur: usize,
    repeat: Repeat,
    mut try_play: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let mut at = cur;
    for _ in 0..len {
        let next = next_index(len, at, repeat)?;
        if try_play(next) {
            return Some(next);
        }
        at = next;
    }
    None
}

/// Fisher-Yates with xorshift. Caller passes any non-zero-ish seed.
pub fn shuffle<T>(items: &mut [T], mut seed: u64) {
    seed |= 1;
    for i in (1..items.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let j = (seed % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

/// `m:ss` for short values, `h:mm:ss` past an hour.
pub fn clock(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// Same, with the original's optional apostrophe minute marker.
pub fn clock_marked(d: Duration, colon: bool) -> String {
    if colon {
        clock(d)
    } else {
        let s = d.as_secs();
        if s >= 3600 {
            clock(d)
        } else {
            format!("{}'{:02}", s / 60, s % 60)
        }
    }
}

/// Natural order: digit runs compare by value, case is ignored.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let (ca, cb) = (a[i], b[j]);
        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            let si = i;
            let sj = j;
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let na: String = a[si..i].iter().collect();
            let nb: String = b[sj..j].iter().collect();
            let va = na.trim_start_matches('0');
            let vb = nb.trim_start_matches('0');
            let ord = va.len().cmp(&vb.len()).then_with(|| va.cmp(vb));
            if ord != Ordering::Equal {
                return ord;
            }
        } else {
            let la = ca.to_ascii_lowercase();
            let lb = cb.to_ascii_lowercase();
            if la != lb {
                return la.cmp(&lb);
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

/// Case-insensitive substring test used by Find in list.
pub fn contains_ci(hay: &str, needle: &str) -> bool {
    needle.is_empty() || hay.to_lowercase().contains(&needle.to_lowercase())
}

/// Bytes as `123 KB` or `1.23 MB`, as the list shows them.
pub fn size_text(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.2} MB", b / (KB * KB))
    } else {
        format!("{} KB", (bytes as f64 / KB).round() as u64)
    }
}

/// Fraction of `pos` in `total`, clamped to 0..=1.
pub fn progress(pos: Duration, total: Duration) -> f64 {
    if total.is_zero() {
        return 0.0;
    }
    (pos.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
}

/// Days since 1970-01-01 to civil date. Howard Hinnant's algorithm.
pub fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

/// Local-ish date (UTC) as `YYYY-MM-DD`, plus month number for month colours.
pub fn date_parts(t: std::time::SystemTime) -> (i32, u32, u32) {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil_from_days(secs.div_euclid(86_400))
}

pub fn date_text(t: std::time::SystemTime) -> String {
    let (y, m, d) = date_parts(t);
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn datetime_text(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, mi) = ((secs / 3600) % 24, (secs / 60) % 60);
    format!("{} {:02}:{:02}", date_text(t), h, mi)
}

/// Month tint index 1..=12 used by the optional month colouring.
pub fn month_tint(month: u32) -> u32 {
    const TINT: [u32; 13] = [
        0, 0xE8F0FA, 0xEAF3EA, 0xF3F6E4, 0xFBF3DF, 0xF7EADF, 0xF7E2E2, 0xF3E4F0, 0xE7E4F4,
        0xE2EAF2, 0xEDE8DE, 0xE8E8EC, 0xE4EEF0,
    ];
    TINT[month.min(12) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
        assert_eq!(date_text(std::time::UNIX_EPOCH + std::time::Duration::from_secs(86_400)), "1970-01-02");
    }

    #[test]
    fn playback_logic() {
        assert_eq!(next_index(3, 0, Repeat::Off), Some(1));
        assert_eq!(next_index(3, 2, Repeat::Off), None);
        assert_eq!(next_index(3, 2, Repeat::All), Some(0));
        assert_eq!(next_index(3, 1, Repeat::One), Some(1));
        assert_eq!(next_index(0, 0, Repeat::All), None);
        assert_eq!(prev_index(3, 0, Repeat::All), 2);
        assert_eq!(prev_index(3, 0, Repeat::Off), 0);
        assert_eq!(Repeat::Off.cycle().cycle().cycle(), Repeat::Off);

        let bad = [false, true, true];
        assert_eq!(advance_playable(3, 0, Repeat::Off, |i| !bad[i]), None);
        assert_eq!(advance_playable(3, 0, Repeat::Off, |i| i == 2), Some(2));
        assert_eq!(advance_playable(3, 0, Repeat::All, |_| false), None);
        assert_eq!(advance_playable(2, 1, Repeat::All, |i| i == 0), Some(0));

        let mut v: Vec<u32> = (0..50).collect();
        shuffle(&mut v, 12345);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, (0..50).collect::<Vec<u32>>());
        assert_ne!(v, (0..50).collect::<Vec<u32>>());

        assert_eq!(clock(Duration::from_secs(615)), "10:15");
        assert_eq!(clock(Duration::from_secs(3615)), "1:00:15");
        assert_eq!(clock_marked(Duration::from_secs(65), false), "1'05");
    }

    #[test]
    fn natural_order_puts_10_after_9() {
        assert_eq!(natural_cmp("track9.mp3", "track10.mp3"), Ordering::Less);
        assert_eq!(natural_cmp("Track9.mp3", "track9.mp3"), Ordering::Equal);
        assert_eq!(natural_cmp("a01", "a1"), Ordering::Equal);
        assert_eq!(natural_cmp("b", "a"), Ordering::Greater);
    }

    #[test]
    fn sizes() {
        assert_eq!(size_text(2048), "2 KB");
        assert_eq!(size_text(3 * 1024 * 1024), "3.00 MB");
        assert_eq!(progress(Duration::from_secs(5), Duration::from_secs(10)), 0.5);
        assert_eq!(progress(Duration::from_secs(5), Duration::ZERO), 0.0);
    }
}
