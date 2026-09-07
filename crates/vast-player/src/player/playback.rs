use rodio::stream::{DeviceSinkBuilder, MixerDeviceSink};
use rodio::{Decoder, Source};
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use super::track::TrackInfo;

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
        let source = Decoder::try_from(file)?;
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
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
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
        self.player
            .try_seek(position)
            .map_err(|e| anyhow::anyhow!("seek failed: {e}"))
    }

    pub fn seek_to_start(&self) -> anyhow::Result<()> {
        self.seek_to(Duration::ZERO)
    }
}
