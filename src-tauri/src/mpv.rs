use interprocess::local_socket::{prelude::*, GenericNamespaced, Stream};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command};

const SOCKET_NAME: &str = "mpvsocket";

/// Запускает mpv с указанным файлом и IPC-сокетом.
///
/// `player_path` — путь к исполняемому файлу плеера.
/// `None` → fallback на `"mpv"` из PATH.
pub fn launch(
    file_path: &str,
    start_position: Option<f64>,
    player_path: Option<&str>,
) -> Result<Child, String> {
    let exe = player_path.unwrap_or("mpv");

    // Защита от «файл был, потом удалили»: проверяем только когда путь явно задан.
    // Для fallback на "mpv" проверку не делаем — PATH сам разрулит.
    if player_path.is_some() {
        let p = std::path::Path::new(exe);
        if !p.is_file() {
            return Err(format!("Файл плеера не найден: {}", exe));
        }
    }

    let mut cmd = Command::new(exe);
    cmd.arg(format!("--input-ipc-server=\\\\.\\pipe\\{}", SOCKET_NAME));
    cmd.arg("--fullscreen");
    cmd.arg("--keep-open=no");
    cmd.arg("--idle=no");

    if let Some(pos) = start_position {
        cmd.arg(format!("--start={}", pos));
    }

    cmd.arg(file_path);

    let child = cmd.spawn().map_err(|e| {
        if player_path.is_some() {
            format!("Не удалось запустить плеер '{}': {}", exe, e)
        } else {
            format!(
                "Не удалось запустить mpv: {}. Убедитесь, что mpv в PATH, или укажите путь в настройках.",
                e
            )
        }
    })?;

    Ok(child)
}

/// Отправляет команду в mpv через IPC и возвращает ответ.
fn send_command(command: &str) -> Result<String, String> {
    // В interprocess 2.x имя создаётся через to_ns_name
    let name = SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| format!("Invalid socket name: {}", e))?;

    // Stream::connect принимает имя
    let mut stream =
        Stream::connect(name).map_err(|e| format!("Не удалось подключиться к mpv IPC: {}", e))?;

    stream
        .write_all(format!("{}\n", command).as_bytes())
        .map_err(|e| format!("Write error: {}", e))?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("Read error: {}", e))?;

    Ok(response)
}

/// Запрашивает у mpv текущую позицию (в секундах).
pub fn get_time_pos() -> Result<f64, String> {
    let response = send_command(r#"{"command":["get_property","time-pos"]}"#)?;
    parse_f64_response(&response)
}

/// Запрашивает у mpv длительность текущего файла (в секундах).
pub fn get_duration() -> Result<f64, String> {
    let response = send_command(r#"{"command":["get_property","duration"]}"#)?;
    parse_f64_response(&response)
}

fn parse_f64_response(response: &str) -> Result<f64, String> {
    let marker = "\"data\":";
    let start = response
        .find(marker)
        .ok_or_else(|| format!("No data field in response: {}", response))?
        + marker.len();

    let rest = &response[start..];
    let end = rest
        .find(|c: char| c == ',' || c == '}')
        .ok_or_else(|| format!("Malformed response: {}", response))?;

    rest[..end]
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("Parse error: {} in '{}'", e, rest))
}

/// Best-effort команда выхода. mpv может быть уже мёртв — это не ошибка.
pub fn quit() -> Result<(), String> {
    // send_command вернёт Err, если пайп недоступен (mpv уже закрыт).
    // Нас это устраивает: значит, гасить нечего.
    let _ = send_command(r#"{"command":["quit"]}"#);
    Ok(())
}
