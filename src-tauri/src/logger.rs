use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn log_path() -> PathBuf {
    // Логи в %APPDATA%\unlexx\lumi\lumi.log
    let dirs = directories::ProjectDirs::from("dev", "unlexx", "lumi")
        .expect("ProjectDirs");
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir).ok();
    data_dir.join("lumi.log")
}

pub fn log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}] {}", timestamp, msg);
    }
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logger::log(&format!($($arg)*))
    };
}