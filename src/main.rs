mod lyrics;
mod player;

use clap::Parser;
use lyrics::Lyrics;
use player::Player;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

slint::include_modules!();

/// Prototype iPod-style audio player.
#[derive(Parser, Debug)]
#[command(about = "Play an audio file with an iPod-style 320x240 UI")]
struct Cli {
    /// Path to the audio file to play (mp3, wav, flac, ogg — whatever rodio's decoder supports)
    audio_file: PathBuf,
}

fn format_time(d: Duration) -> String {
    let total_secs = d.as_secs();
    format!("{}:{:02}", total_secs / 60, total_secs % 60)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.audio_file.exists() {
        anyhow::bail!("Audio file not found: {}", cli.audio_file.display());
    }

    // Load lyrics up front. Missing lyrics is a normal, expected outcome,
    // not an error — the UI has an explicit "no lyrics" state for it.
    let lyrics = Lyrics::find_and_load_for(&cli.audio_file);
    let has_lyrics = lyrics.is_some();

    // play_file() is the single entry point for starting playback.
    let player = Rc::new(Player::play_file(&cli.audio_file)?);

    let ui = MainWindow::new()?;
    ui.set_track_title(player.info.title.clone().into());
    ui.set_track_artist(player.info.artist.clone().into());
    ui.set_has_lyrics(has_lyrics);

    // Drive UI updates from a timer tick rather than blocking the UI
    // thread — position/lyrics/progress all get refreshed here.
    let ui_weak = ui.as_weak();
    let player_for_timer = player.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };

            let elapsed = player_for_timer.elapsed();
            ui.set_is_playing(player_for_timer.is_playing());

            if let Some(total) = player_for_timer.total_duration() {
                let frac = (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0);
                ui.set_progress(frac);
                ui.set_time_label(
                    format!("{} / {}", format_time(elapsed), format_time(total)).into(),
                );
            } else {
                // Unknown total duration: still show elapsed time, just
                // no meaningful progress fraction.
                ui.set_progress(0.0);
                ui.set_time_label(format_time(elapsed).into());
            }

            if let Some(lyrics) = &lyrics {
                if let Some(line) = lyrics.current_line(elapsed.as_secs_f64()) {
                    ui.set_lyric_line(line.into());
                }
            }
        },
    );

    ui.run()?;
    Ok(())
}
