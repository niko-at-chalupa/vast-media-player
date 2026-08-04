use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use audiotags::Tag;

#[derive(Debug, Clone, Default)]
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
}

/// Owns the audio output stream + sink for a currently loaded track.
/// Must be kept alive for the duration of playback (dropping it stops
/// the audio), so hang on to the `Player` for as long as you want sound.
pub struct Player {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Sink,
    started_at: Instant,
    total_duration: Option<Duration>,
    pub info: TrackInfo,
}

impl Player {
    /// Loads and immediately begins playing the given audio file.
    /// This is the single function the rest of the app should call to
    /// start playback — extend internals (crossfade, gapless, EQ, etc.)
    /// behind this signature as the project grows.
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
            started_at: Instant::now(),
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

    /// Elapsed playback time. Naive (doesn't account for seeking, since
    /// this prototype doesn't support seeking yet) but fine as a stand-in
    /// until real position tracking is added.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Total track duration, if the decoder was able to determine it.
    /// Not all formats/containers expose this reliably — treat None as
    /// a normal case to handle in the UI (e.g. hide/zero the progress bar).
    pub fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }
}
