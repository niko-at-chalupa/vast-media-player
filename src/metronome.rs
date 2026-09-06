use std::time::Duration;

pub fn apply_tempo_to_ui(ui: &crate::MainWindow, tempo: Option<u32>) {
    match tempo {
        Some(bpm) if bpm > 0 => {
            let beat_duration = Duration::from_secs_f64(60.0 / bpm as f64);
            ui.global::<crate::PlayerData>().set_has_tempo(true);
            ui.global::<crate::PlayerData>().set_tempo(bpm as i32);
            ui.global::<crate::PlayerData>()
                .set_duration_between_beats(beat_duration.as_millis() as i64);
        }
        _ => {
            ui.global::<crate::PlayerData>().set_has_tempo(false);
            ui.global::<crate::PlayerData>().set_tempo(0);
            ui.global::<crate::PlayerData>()
                .set_duration_between_beats(0);
        }
    }
}

pub fn beat_index(elapsed: Duration, tempo: Option<u32>, beats_per_measure: u8) -> i32 {
    let Some(bpm) = tempo.filter(|bpm| *bpm > 0) else {
        return -1;
    };

    ((elapsed.as_nanos() * bpm as u128 / 60_000_000_000) % beats_per_measure as u128) as i32
}