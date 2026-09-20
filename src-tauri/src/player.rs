use crate::cache;
use crate::mpv;
use std::time::Duration;
use tauri::Emitter;

#[tauri::command]
pub async fn play_video(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let file_path = path.clone();
    std::thread::spawn(move || {
        if let Err(e) = play_and_track(&file_path, app) {
            eprintln!("Playback error: {}", e);
        }
    });
    Ok(())
}

fn play_and_track(file_path: &str, app: tauri::AppHandle) -> Result<(), String> {
      let mut child = mpv::launch(file_path)?;

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
                eprintln!("Error waiting for mpv: {}", e);
                break;
            }
        }
    }

    println!(
        "Playback finished: {} — position {:.1}s / duration {:.1}s",
        file_path, last_position, last_duration
    );

    // Сохраняем в базу
    if let Ok(conn) = cache::init_db() {
        if let Err(e) = cache::mark_watched(&conn, file_path, last_position, last_duration) {
            eprintln!("Failed to save watch status: {}", e);
        } else {
            let watched = last_duration > 0.0 && last_position / last_duration >= 0.95;
            println!(
                "  → Marked as {}",
                if watched { "WATCHED" } else { "in progress" }
            );
if let Ok(conn) = cache::init_db() {
    if let Err(e) = cache::mark_watched(&conn, file_path, last_position, last_duration) {
        eprintln!("Failed to save watch status: {}", e);
    } else {
        let watched = last_duration > 0.0 && last_position / last_duration >= 0.95;
        println!(
            "  → Marked as {} ({:.1}%)",
            if watched { "WATCHED" } else { "in progress" },
            if last_duration > 0.0 {
                last_position / last_duration * 100.0
            } else {
                0.0
            }
        );

        // Эмитим событие во фронтенд
        app.emit(
            "watch_status_updated",
            serde_json::json!({
                "path": file_path,
                "watched": watched,
            }),
        )
        .ok();
    }
}
        }
    }

    Ok(())
}