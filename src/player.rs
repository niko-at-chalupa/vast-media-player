use anyhow::Context;
use audiotags::Tag;
use rodio::stream::{DeviceSinkBuilder, MixerDeviceSink};
use rodio::{Decoder, Source};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use std::fmt;

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct TrackInfo {
    pub title: String,
    pub artists: Vec<String>,
    pub path: PathBuf,
}

impl TrackInfo {
    pub fn from_path(path: &Path) -> Self {
        let filename_title = match path.file_stem() {
            Some(s) => s.to_string_lossy().to_string(),
            None => "[no title]".to_string(),
        };

        let tags = Tag::default().read_from_path(&path);

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

        TrackInfo {
            title,
            artists,
            path: path.to_path_buf(),
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

pub struct Player {
    _device_sink: MixerDeviceSink,
    player: rodio::Player,
    total_duration: Option<Duration>,
    pub info: TrackInfo,
}

impl Player {
    pub fn play_file(path: &Path) -> anyhow::Result<Player> {
        let mut device_sink = DeviceSinkBuilder::open_default_sink()?;
        device_sink.log_on_drop(false);
        let player = rodio::Player::connect_new(&device_sink.mixer());

        let file = File::open(path)?;
        let source = Decoder::try_from(file)?;   // sets byte_len from metadata
        let total_duration = source.total_duration();
        player.append(source);

        Ok(Player {
            _device_sink: device_sink,
            player,
            total_duration,
            info: TrackInfo::from_path(path),
        })
    }

    pub fn toggle_pause(&self) {
        if self.player.is_paused() { self.player.play(); } else { self.player.pause(); }
    }
    
    pub fn set_pause(&self, is_paused: bool) {
        match is_paused {
            true => self.player.pause(),
            false => self.player.play(),
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.player.is_paused() && !self.player.empty()
    }

    pub fn is_finished(&self) -> bool {
        self.player.empty()
    }

    pub fn elapsed(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }

    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn seek_to(&self, position: Duration) -> anyhow::Result<()> {
        self.player.try_seek(position).map_err(|e| anyhow::anyhow!("seek failed: {e}"))
    }

    pub fn seek_to_start(&self) -> anyhow::Result<()> {
        self.seek_to(Duration::ZERO)
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct TrackId(u64);

pub struct Queue {
    track_order: Vec<TrackId>,
    tracks: HashMap<TrackId, TrackInfo>,
    player: Option<Rc<RefCell<Player>>>,
    current_index: Option<usize>,
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            track_order: Vec::new(),
            tracks: HashMap::new(),
            player: None,
            current_index: None,
        }
    }
}

#[allow(dead_code)]
impl Queue {
    pub fn track_order(&self) -> &Vec<TrackId> {
        &self.track_order
    }

    pub fn tracks(&self) -> &HashMap<TrackId, TrackInfo> {
        &self.tracks
    }

    pub fn player(&self) -> &Option<Rc<RefCell<Player>>> {
        &self.player
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn clear_tracks(&mut self) {
        self.tracks.clear();
        self.track_order.clear();
    }

    pub fn insert_tracks(&mut self, tracks: Vec<TrackInfo>) {
        for track in tracks {
            let track_id: TrackId;
            {
                let mut hasher = fxhash::FxHasher64::default();
                track.hash(&mut hasher);
                track_id = TrackId(hasher.finish());
            }
            self.tracks.insert(track_id.clone(), track);
            self.track_order.push(track_id);
        }
    }

    pub fn play_at(&mut self, index: usize) -> anyhow::Result<()> {
        let track_id = self
            .track_order
            .get(index)
            .context("Index out of bounds")?
            .clone();
        let track = self
            .tracks
            .get(&track_id)
            .context("Queue does not contain track")?;

        if let Some(old_player) = self.player.take() {
            old_player.borrow_mut().stop();
        }

        let player = track.play()?;

        self.player = Some(Rc::new(RefCell::new(player)));
        self.current_index = Some(index);

        Ok(())
    }

    pub fn play_at_clamped(&mut self, index: usize) -> anyhow::Result<bool> {
        let last = self.track_order.len().saturating_sub(1);
        self.play_at(index.min(last))?;
        Ok(true)
    }

    pub fn play_next(&mut self) -> anyhow::Result<bool> {
        let next_index = match self.current_index {
            Some(i) => i + 1,
            None => 0, // nothing playing yet, start from the beginning
        };

        if next_index < self.track_order.len() {
            self.play_at(next_index)?;
            Ok(true)
        } else {
            self.player = None;
            self.current_index = None;
            Ok(false)
        }
    }

    pub fn play_previous(&mut self) -> anyhow::Result<bool> {
        match self.current_index {
            Some(i) if i > 0 => {
                self.play_at(i - 1)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn currently_playing(&self) -> Option<&TrackId> {
        let idx = self.current_index?;
        self.track_order.get(idx)
    }

    pub fn currently_playing_info(&self) -> Option<&TrackInfo> {
        let id = self.currently_playing()?;
        self.tracks.get(id)
    }
}

impl fmt::Display for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Queue ({} tracks):", self.track_order.len())?;
        for (index, track_id) in self.track_order.iter().enumerate() {
            if let Some(track) = self.tracks.get(track_id) {
                let full_hash = format!("{:016x}", track_id.0);
                let truncated_hash = &full_hash[..7.min(full_hash.len())];
                
                let is_current = self.current_index == Some(index);
                let label = if is_current { " [PLAYING]" } else { "" };

                writeln!(f, "  {}. {} ({}){}", index + 1, track.title, truncated_hash, label)?;
            }
        }
        Ok(())
    }
}