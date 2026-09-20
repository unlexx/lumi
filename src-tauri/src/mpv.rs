use interprocess::local_socket::{
    prelude::*,
    GenericNamespaced,
    Stream,
};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command};
use std::time::Duration;

const SOCKET_NAME: &str = "mpvsocket";

/// Запускает mpv с указанным файлом и IPC-сокетом.
pub fn launch(file_path: &str) -> Result<Child, String> {
    let child = Command::new("mpv")
        .arg(format!("--input-ipc-server=\\\\.\\pipe\\{}", SOCKET_NAME))
        .arg(file_path)
        .spawn()
        .map_err(|e| format!("Не удалось запустить mpv: {}. Убедитесь, что mpv в PATH.", e))?;

    Ok(child)
}

/// Отправляет команду в mpv через IPC и возвращает ответ.
fn send_command(command: &str) -> Result<String, String> {
    // В interprocess 2.x имя создаётся через to_ns_name
    let name = SOCKET_NAME
        .to_ns_name::<GenericNamespaced>()
        .map_err(|e| format!("Invalid socket name: {}", e))?;

    // Stream::connect принимает имя
    let mut stream = Stream::connect(name)
        .map_err(|e| format!("Не удалось подключиться к mpv IPC: {}", e))?;

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

/// Ждёт завершения процесса mpv с таймаутом. Возвращает true, если процесс завершился.
pub fn wait_for_exit(child: &mut Child, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(timeout_secs) {
                    return false;
                }
                std::thread::sleep(Duration::from_millis(500));
            }
            Err(_) => return true,
        }
    }
}