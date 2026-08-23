mod lyrics;
mod player;
mod status_bar;

use anyhow::Context;
use clap::Parser;
use lyrics::Lyrics;
use player::Player;
use slint::SharedString;
use souvlaki::{MediaControlEvent, MediaControls, PlatformConfig};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tracing::{error};
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
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();

    if !cli.audio_file.exists() {
        anyhow::bail!("Audio file not found: {}", cli.audio_file.display());
    }

    let queue: Rc<RefCell<Queue>> = Rc::new(RefCell::new(Queue::default()));
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
        queue
            .borrow()
            .player()
            .clone()
            .context("Player should exist after playing track")?
    };

    let lyrics: Option<Lyrics> = Lyrics::find_and_load_for(&player.borrow().info.path);

    let ui = MainWindow::new()?;
    ui.global::<PlayerData>()
        .set_track_title(player.borrow().info.title.clone().into());

    {
        let artists = player.borrow().info.artists.clone();
        let artists_string = if !artists.is_empty() {
            artists.join(", ")
        } else {
            "[no artist tags]".to_string()
        };
        ui.global::<PlayerData>()
            .set_track_artist(SharedString::from(artists_string));
    }

    ui.global::<PlayerData>().set_has_lyrics(lyrics.is_some());

    let mut controls = MediaControls::new(PlatformConfig {
        dbus_name: "vast-media-player",
        display_name: "Vast Media Player",
        hwnd: None,
    })?;

    let (mpris_tx, mpris_rx): (_, Receiver<MediaControlEvent>) = channel();
    controls.attach(move |event| match event {
        e @ (MediaControlEvent::Play 
        | MediaControlEvent::Pause 
        | MediaControlEvent::Toggle
        | MediaControlEvent::Next 
        | MediaControlEvent::Previous) => {
            let _ = mpris_tx.send(e); 
        }
        _ => {}
    })?;

    let ui_weak = ui.as_weak();
    let queue_for_timer = queue.clone();
    let mut lyrics_for_timer = lyrics;
    let mut last_index = queue_for_timer.borrow().current_index();

    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };

            let player = {
                let q = queue_for_timer.borrow();
                match q.player() {
                    Some(p) => p.clone(),
                    None => return,
                }
            };

            let (elapsed, is_playing, is_finished, total) = {
                let p = player.borrow();
                (
                    p.elapsed(),
                    p.is_playing(),
                    p.is_finished(),
                    p.total_duration(),
                )
            };

            let restart_on_previous: bool;
            const RESTART_THRESHOLD: Duration = Duration::from_secs(4);
            if elapsed > RESTART_THRESHOLD {
                restart_on_previous = true;
            } else {
                restart_on_previous = false;
            }

            if let Ok(msg) = mpris_rx.try_recv() {
                match msg {
                    MediaControlEvent::Play => player.borrow().set_pause(false),
                    MediaControlEvent::Pause => player.borrow().set_pause(true),
                    MediaControlEvent::Toggle => player.borrow().toggle_pause(),
                    MediaControlEvent::Next => { 
                        ui.global::<PlayerData>().set_extra_info("".into());
                        let _ = queue_for_timer.borrow_mut().play_next();
                    },
                    MediaControlEvent::Previous => {
                        if restart_on_previous {
                            if let Err(e) = player.borrow().seek_to_start() {
                                ui.global::<PlayerData>().set_extra_info("".into());
                                ui.global::<PlayerData>().set_extra_info("error seeking to start; went to previous track".into());
                                error!("{:?}", e);
                                let _ = queue_for_timer.borrow_mut().play_previous();
                            } else {
                                ui.global::<PlayerData>().set_extra_info("error seeking to start; went to previous track".into());
                                ui.global::<PlayerData>().set_extra_info("seeked to start; press ⏮ to go to previous track".into());
                            }
                        } else {
                            let _ = queue_for_timer.borrow_mut().play_previous();
                        }
                    },
                    _ => ()
                }
            }

            ui.global::<PlayerData>().set_is_playing(is_playing);

            if let Some(total) = total {
                let frac = (elapsed.as_secs_f32() / total.as_secs_f32()).min(1.0);
                ui.global::<PlayerData>().set_progress(frac);
                ui.global::<PlayerData>().set_time_label(
                    format!("{} / {}", format_time(elapsed), format_time(total)).into(),
                );
            } else {
                ui.global::<PlayerData>().set_progress(0.0);
                ui.global::<PlayerData>()
                    .set_time_label(format_time(elapsed).into());
            }

            if let Some(lyrics) = &lyrics_for_timer {
                if let Some(line) = lyrics.current_line(elapsed.as_secs_f64()) {
                    ui.global::<PlayerData>().set_lyric_line(line.into());
                }
            }

            let current_index = queue_for_timer.borrow().current_index();
            if current_index != last_index {
                last_index = current_index;
                if let Some(p) = queue_for_timer.borrow().player() {
                    let p = p.borrow();
                    ui.global::<PlayerData>()
                        .set_track_title(p.info.title.clone().into());
                    let artists_string = if !p.info.artists.is_empty() {
                        p.info.artists.join(", ")
                    } else {
                        "[no artist tags]".to_string()
                    };
                    ui.global::<PlayerData>()
                        .set_track_artist(SharedString::from(artists_string));

                    lyrics_for_timer = Lyrics::find_and_load_for(&p.info.path);
                    ui.global::<PlayerData>()
                        .set_has_lyrics(lyrics_for_timer.is_some());
                    ui.global::<PlayerData>()
                        .set_lyric_line("".into());
                }
            }

            if is_finished {
                let _ = queue_for_timer.borrow_mut().play_next();
            }

            crate::status_bar::populate_status_bar(&ui)
        },
    );

    ui.run()?;
    Ok(())
}
