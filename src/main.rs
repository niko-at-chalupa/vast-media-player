mod lyrics;
mod player;
mod status_bar;

use anyhow::Context;
use clap::Parser;
use lyrics::Lyrics;
use player::Player;
use slint::SharedString;
// use slint::platform::Key::P;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::rc::Rc;
use std::time::Duration;
use souvlaki::{MediaControls, MediaControlEvent, PlatformConfig};
use std::cell::RefCell;

use crate::player::{Queue, TrackInfo};

#[derive(Parser, Debug)]
#[command(about = "Play an audio file")]
struct Cli {
    audio_file: PathBuf,
}

fn format_time(d: Duration) -> String {
    let total_secs = d.as_secs();
    format!("{}:{:02}", total_secs / 60, total_secs % 60)
}

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.audio_file.exists() {
        anyhow::bail!("Audio file not found: {}", cli.audio_file.display());
    }

    let queue: Rc<RefCell<Queue>> = Rc::new(RefCell::new(
        Queue::default()
    ));
    {
        let tracks: Vec<TrackInfo>;
        if cli.audio_file.is_dir() {
            tracks = TrackInfo::from_dir(&cli.audio_file)?;
        } else {
            tracks = vec![TrackInfo::from_path(&cli.audio_file)];
        }
        queue.borrow_mut().insert_tracks(tracks);
    }

    let player: Rc<RefCell<Player>> = {
        queue.borrow_mut().play_at(0)?;
        queue.borrow().player().clone().context("Player should exist after playing track")?
    };

    let mut lyrics: Option<Lyrics> = Lyrics::find_and_load_for(&player.borrow().info.path);

    let ui = MainWindow::new()?;
    ui.global::<PlayerData>().set_track_title(player.borrow().info.title.clone().into());

    {
        let artists = player.borrow().info.artists.clone();
        let artists_string = if !artists.is_empty() {
            artists.join(", ")
        } else {
            "[no artist tags]".to_string()
        };
        ui.global::<PlayerData>().set_track_artist(SharedString::from(artists_string));
    }

    ui.global::<PlayerData>().set_has_lyrics(lyrics.is_some());

    let mut controls = MediaControls::new(PlatformConfig {
        dbus_name: "vast-media-player",
        display_name: "Vast Media Player",
        hwnd: None,
    })?;

    let (mpris_tx, mpris_rx): (_, Receiver<()>) = channel();
    controls.attach(move |event| match event {
        MediaControlEvent::Play | MediaControlEvent::Pause | MediaControlEvent::Toggle => {
            let _ = mpris_tx.send(());
        }
        _ => {}
    })?;

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

            let elapsed = player_for_timer.borrow().elapsed();
            ui.global::<PlayerData>().set_is_playing(player_for_timer.borrow().is_playing());

            if let Some(total) = player_for_timer.borrow().total_duration() {
                let frac = (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0);
                ui.global::<PlayerData>().set_progress(frac);
                ui.global::<PlayerData>().set_time_label(
                    format!("{} / {}", format_time(elapsed), format_time(total)).into(),
                );
            } else {
                ui.global::<PlayerData>().set_progress(0.0);
                ui.global::<PlayerData>().set_time_label(format_time(elapsed).into());
            }

            if let Some(lyrics) = &lyrics {
                if let Some(line) = lyrics.current_line(elapsed.as_secs_f64()) {
                    ui.global::<PlayerData>().set_lyric_line(line.into());
                }
            }

            if mpris_rx.try_recv().is_ok() {
                player_for_timer.borrow().toggle_pause();
            }

            crate::status_bar::populate_status_bar(&ui)
        },
    );

    ui.run()?;
    Ok(())
}
