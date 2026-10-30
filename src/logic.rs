use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    #[default]
    Off,
    All,
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
            Repeat::Off => "off",
            Repeat::All => "all",
            Repeat::One => "one",
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

/// Index before `cur`. Stays at 0 at the top of the list.
pub fn prev_index(cur: usize) -> usize {
    cur.saturating_sub(1)
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

/// `m:ss` for display. Hours roll into minutes.
pub fn mmss(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_logic() {
        // Next and repeat.
        assert_eq!(next_index(3, 0, Repeat::Off), Some(1));
        assert_eq!(next_index(3, 2, Repeat::Off), None);
        assert_eq!(next_index(3, 2, Repeat::All), Some(0));
        assert_eq!(next_index(3, 1, Repeat::One), Some(1));
        assert_eq!(next_index(0, 0, Repeat::All), None);
        assert_eq!(prev_index(0), 0);
        assert_eq!(prev_index(2), 1);
        assert_eq!(Repeat::Off.cycle().cycle().cycle(), Repeat::Off);

        // Skip cap: bad files are passed over, but a full bad pass ends.
        let bad = [false, true, true];
        assert_eq!(advance_playable(3, 0, Repeat::Off, |i| !bad[i]), None);
        assert_eq!(advance_playable(3, 0, Repeat::Off, |i| i == 2), Some(2));
        assert_eq!(advance_playable(3, 0, Repeat::All, |_| false), None);
        assert_eq!(advance_playable(2, 1, Repeat::All, |i| i == 0), Some(0));

        // Shuffle keeps every item.
        let mut v: Vec<u32> = (0..50).collect();
        shuffle(&mut v, 12345);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, (0..50).collect::<Vec<u32>>());
        assert_ne!(v, (0..50).collect::<Vec<u32>>());

        assert_eq!(mmss(Duration::from_secs(615)), "10:15");
    }
}
