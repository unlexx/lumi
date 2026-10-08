use crate::cache;
use crate::cache::AppState;
use crate::mpv;
use std::time::Duration;
use tauri::{Emitter, Manager};

/// RAII-guard: гарантирует сброс флага player_active при выходе из области,
/// включая панику. Сбрасывается раньше, чем `child.kill()`/`drop`.
struct PlayerActiveGuard<'a>(&'a AppState);

impl<'a> PlayerActiveGuard<'a> {
    fn new(state: &'a AppState) -> Self {
        state.set_player_active(true);
        Self(state)
    }
}

impl<'a> Drop for PlayerActiveGuard<'a> {
    fn drop(&mut self) {
        self.0.set_player_active(false);
    }
}

/// RAII-guard: гарантирует сброс player_active и зачистку mpv_child
/// при выходе из области, включая панику.
struct PlayerSession<'a> {
    state: &'a AppState,
}

impl<'a> PlayerSession<'a> {
    fn new(state: &'a AppState) -> Self {
        state.set_player_active(true);
        Self { state }
    }
}

impl<'a> Drop for PlayerSession<'a> {
    fn drop(&mut self) {
        // Забираем child (если остался) и убиваем его.
        // Если play_and_track уже сам забрал и дождался — здесь None, ничего не делаем.
        if let Some(mut child) = self.state.take_mpv_child() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.state.set_player_active(false);
    }
}

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
    let state = app.state::<AppState>();
    let _session = PlayerSession::new(&state);

    let child = mpv::launch(file_path, start_position)?;
    state.set_mpv_child(child);

    std::thread::sleep(Duration::from_millis(2000));

    let mut last_position: f64 = 0.0;
    let mut last_duration: f64 = 0.0;
    let start = std::time::Instant::now();

    loop {
        // Проверяем статус mpv под коротким lock'ом
        let exited = {
            let mut guard = state.mpv_child.lock().expect("mpv_child mutex poisoned");
            match guard.as_mut() {
                Some(child) => match child.try_wait() {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(e) => {
                        crate::log_info!("Error waiting for mpv: {}", e);
                        true
                    }
                },
                None => true, // кто-то уже забрал — считаем, что вышли
            }
        };

        if exited {
            break;
        }

        if let Ok(pos) = mpv::get_time_pos() {
            last_position = pos;
        }
        if let Ok(dur) = mpv::get_duration() {
            last_duration = dur;
        }

        if start.elapsed() > Duration::from_secs(6 * 60 * 60) {
            // 6 часов — принудительно гасим
            if let Some(mut child) = state.take_mpv_child() {
                let _ = child.kill();
            }
            break;
        }

        std::thread::sleep(Duration::from_secs(2));
    }

    // mpv завершился сам — забираем handle из state, чтобы Drop-guard его не убивал повторно
    let _ = state.take_mpv_child();

    crate::log_info!(
        "Playback finished: {} — position {:.1}s / duration {:.1}s",
        file_path,
        last_position,
        last_duration
    );

    let watched = {
        let conn = state.conn();

        if let Err(e) = cache::mark_watched(&conn, file_path, last_position, last_duration) {
            crate::log_info!("Failed to save watch status: {}", e);
            return Ok(());
        }

        last_duration > 0.0 && last_position / last_duration >= 0.95
    };

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
