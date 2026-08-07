use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use audiotags::Tag;
use std::hash::{Hash, Hasher};

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
                    if matches!(ext_lower.as_str(), "mp3" | "flac" | "wav" | "ogg" | "oga" | "m4a" | "mp4") {
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

pub struct Player {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Sink,
    total_duration: Option<Duration>,
    pub info: TrackInfo,
}

impl Player {
    pub fn play_file(path: &Path) -> anyhow::Result<Player> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        let file = BufReader::new(File::open(path)?);
        let source = Decoder::new(file)?;
        let total_duration = source.total_duration();
        sink.append(source);

        Ok(Player {
            _stream: stream,
            _stream_handle: stream_handle,
            sink,
            total_duration,
            info: TrackInfo::from_path(path),
        })
    }

    pub fn toggle_pause(&self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }

    pub fn is_playing(&self) -> bool {
        !self.sink.is_paused() && !self.sink.empty()
    }

    pub fn is_finished(&self) -> bool {
        self.sink.empty()
    }

    pub fn elapsed(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct TrackId(u64);

pub struct Queue {
    track_order: Vec<TrackId>,
    tracks: HashMap<TrackId, TrackInfo>,
    player: Option<Player>,
}

impl Queue {
    pub fn track_order(&self) -> &Vec<TrackId> {
        &self.track_order
    }

    pub fn tracks(&self) -> &HashMap<TrackId, TrackInfo> {
        &self.tracks
    }

    pub fn player(&self) -> &Option<Player> {
        &self.player
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
}