use crate::cache;
use crate::cache::AppState;
use crate::mpv;
use std::time::Duration;
use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn play_video(
    app: tauri::AppHandle,
    path: String,
    start_position: Option<f64>,
) -> Result<(), String> {
    let file_path = path.clone();
    std::thread::spawn(move || {
        if let Err(e) = play_and_track(&file_path, start_position, app) {
            crate::log_info!("Playback error: {}", e);
        }
    });
    Ok(())
}

fn play_and_track(
    file_path: &str,
    start_position: Option<f64>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut child = mpv::launch(file_path, start_position)?;

    std::thread::sleep(Duration::from_millis(2000));

    let mut last_position: f64 = 0.0;
    let mut last_duration: f64 = 0.0;
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if let Ok(pos) = mpv::get_time_pos() {
                    last_position = pos;
                }
                if let Ok(dur) = mpv::get_duration() {
                    last_duration = dur;
                }

                if start.elapsed() > Duration::from_secs(6 * 60 * 60) {
                    let _ = child.kill();
                    break;
                }

                std::thread::sleep(Duration::from_secs(2));
            }
            Err(e) => {
                crate::log_info!("Error waiting for mpv: {}", e);
                break;
            }
        }
    }

    crate::log_info!(
        "Playback finished: {} — position {:.1}s / duration {:.1}s",
        file_path,
        last_position,
        last_duration
    );

    // Записываем статус в БД, держим lock только на время записи
    let watched = {
        let state = app.state::<AppState>();
        let conn = state.conn();

        if let Err(e) = cache::mark_watched(&conn, file_path, last_position, last_duration) {
            crate::log_info!("Failed to save watch status: {}", e);
            return Ok(());
        }

        last_duration > 0.0 && last_position / last_duration >= 0.95
    }; // lock отпущен здесь

    crate::log_info!(
        "  → Marked as {} ({:.1}%)",
        if watched { "WATCHED" } else { "in progress" },
        if last_duration > 0.0 {
            last_position / last_duration * 100.0
        } else {
            0.0
        }
    );

    app.emit(
        "watch_status_updated",
        serde_json::json!({
            "path": file_path,
            "watched": watched,
            "position": last_position,
            "duration": last_duration,
        }),
    )
    .ok();

    Ok(())
}
