use audiotags::Tag;
use serde_json::Value;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::playback::Player;

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct TrackInfo {
    pub title: String,
    pub artists: Vec<String>,
    pub path: PathBuf,
    pub tempo: Option<u32>,
}

impl TrackInfo {
    fn tempo_from_ffprobe_json(output: &str) -> Option<u32> {
        let value: Value = serde_json::from_str(output).ok()?;

        fn find_tempo(value: &Value) -> Option<u32> {
            match value {
                Value::Object(map) => {
                    for (key, item) in map {
                        let normalized = key.to_ascii_lowercase();
                        if matches!(
                            normalized.as_str(),
                            "tempo" | "tbpm" | "bpm" | "beats_per_minute"
                        ) {
                            if let Some(tempo) = item.as_str().and_then(|s| s.parse::<f64>().ok()) {
                                return Some(tempo.round() as u32);
                            }
                            if let Some(tempo) = item.as_i64() {
                                return Some(tempo as u32);
                            }
                            if let Some(tempo) = item.as_f64() {
                                return Some(tempo.round() as u32);
                            }
                        }
                    }

                    for item in map.values() {
                        if let Some(tempo) = find_tempo(item) {
                            return Some(tempo);
                        }
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        if let Some(tempo) = find_tempo(item) {
                            return Some(tempo);
                        }
                    }
                }
                _ => {}
            }

            None
        }

        find_tempo(&value)
    }

    fn tempo_for_path(path: &Path) -> Option<u32> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format_tags:stream_tags",
                "-of",
                "json",
                path.to_string_lossy().as_ref(),
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        Self::tempo_from_ffprobe_json(&String::from_utf8_lossy(&output.stdout))
    }

    pub fn from_path(path: &Path) -> Self {
        let filename_title = match path.file_stem() {
            Some(s) => s.to_string_lossy().to_string(),
            None => "[no title]".to_string(),
        };

        let tags = Tag::default().read_from_path(path);

        let (title, artists) = match tags {
            Ok(tags) => {
                let title = match tags.title() {
                    Some(t) if !t.is_empty() => t.to_string(),
                    _ => filename_title,
                };

                let mut artists: Vec<String> = Vec::new();
                for a in tags.artists().unwrap_or(vec![]) {
                    artists.push(a.to_string());
                }
                (title, artists)
            }
            Err(_) => (filename_title, vec![]),
        };

        let tempo = Self::tempo_for_path(path);

        TrackInfo {
            title,
            artists,
            path: path.to_path_buf(),
            tempo,
        }
    }

    pub fn from_dir(path: &Path) -> anyhow::Result<Vec<Self>> {
        let mut tracks = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if matches!(
                        ext_lower.as_str(),
                        "mp3" | "flac" | "wav" | "ogg" | "oga" | "m4a" | "mp4"
                    ) {
                        tracks.push(Self::from_path(&path));
                    }
                }
            }
        }
        Ok(tracks)
    }

    pub fn play(&self) -> anyhow::Result<Player> {
        Player::play_file(&self.path)
    }
}

impl fmt::Display for TrackInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}
