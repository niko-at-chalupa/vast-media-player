//! Minimal .lrc (synced lyrics) parsing.
//!
//! An .lrc file looks like:
//!   [00:12.34]First line of lyrics
//!   [00:15.10]Second line
//!
//! This module is intentionally simple/naive — good enough for a
//! prototype, and easy to swap out later for something more complete
//! (multi-timestamp lines, metadata tags, word-level sync, etc).

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time_secs: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
}

impl Lyrics {
    /// Given an audio file path, look for a sibling .lrc file with the
    /// same base name (e.g. `song.mp3` -> `song.lrc`) and parse it if found.
    /// Returns None if no lyrics file exists — callers should treat that
    /// as a normal, expected case, not an error.
    pub fn find_and_load_for(audio_path: &Path) -> Option<Lyrics> {
        let lrc_path = audio_path.with_extension("lrc");
        if !lrc_path.exists() {
            return None;
        }
        let contents = fs::read_to_string(&lrc_path).ok()?;
        Some(Lyrics::parse(&contents))
    }

    pub fn parse(contents: &str) -> Lyrics {
        let mut lines = Vec::new();

        for raw_line in contents.lines() {
            let raw_line = raw_line.trim();
            if raw_line.is_empty() {
                continue;
            }

            // A line can have multiple leading timestamps, e.g. [00:12.00][00:45.00]Text
            let mut rest = raw_line;
            let mut timestamps = Vec::new();

            while rest.starts_with('[') {
                if let Some(close) = rest.find(']') {
                    let tag = &rest[1..close];
                    if let Some(secs) = parse_timestamp(tag) {
                        timestamps.push(secs);
                    }
                    // if it's a metadata tag like [ar:Artist], parse_timestamp
                    // returns None and we just skip it.
                    rest = &rest[close + 1..];
                } else {
                    break;
                }
            }

            let text = rest.trim().to_string();
            for t in timestamps {
                lines.push(LyricLine {
                    time_secs: t,
                    text: text.clone(),
                });
            }
        }

        lines.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap());
        Lyrics { lines }
    }

    /// Returns the text of the most recent lyric line at or before `time_secs`.
    pub fn current_line(&self, time_secs: f64) -> Option<&str> {
        self.lines
            .iter()
            .rev()
            .find(|l| l.time_secs <= time_secs)
            .map(|l| l.text.as_str())
    }
}

/// Parses "mm:ss.xx" or "mm:ss" into seconds. Returns None for anything
/// that isn't a timestamp (e.g. metadata tags like "ar:Some Artist").
fn parse_timestamp(tag: &str) -> Option<f64> {
    let (min_str, rest) = tag.split_once(':')?;
    let minutes: f64 = min_str.trim().parse().ok()?;
    let seconds: f64 = rest.trim().parse().ok()?;
    Some(minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_lrc() {
        let sample = "[00:00.00]Intro\n[00:12.34]First line\n[00:15.10]Second line\n";
        let lyrics = Lyrics::parse(sample);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.current_line(13.0), Some("First line"));
        assert_eq!(lyrics.current_line(0.0), Some("Intro"));
        assert_eq!(lyrics.current_line(20.0), Some("Second line"));
    }

    #[test]
    fn skips_metadata_tags() {
        let sample = "[ar:Some Artist]\n[00:01.00]Hello\n";
        let lyrics = Lyrics::parse(sample);
        assert_eq!(lyrics.lines.len(), 1);
        assert_eq!(lyrics.lines[0].text, "Hello");
    }
}
