use anyhow::Context;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::{Player, TrackInfo};

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
            None => 0,
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

                writeln!(
                    f,
                    "  {}. {} ({}){}",
                    index + 1,
                    track.title,
                    truncated_hash,
                    label
                )?;
            }
        }
        Ok(())
    }
}
