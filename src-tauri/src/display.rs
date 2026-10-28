//! Adaptive refresh rate: подгоняем Hz монитора/проектора под fps видео.
//!
//! Всё под Windows. Ошибки WinAPI логируются и не прерывают воспроизведение —
//! плеер просто стартует на текущей частоте.

use std::mem;
use std::process::Command;
use std::sync::Mutex;

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplaySettingsW, CDS_FULLSCREEN, DEVMODEW,
    DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH,
    ENUM_CURRENT_SETTINGS,
};

/// Частоты, которые имеет смысл запрашивать у драйвера.
/// Драйверы обычно отдают 23.976 как 23, 29.97/59.94 как 30/60 — поэтому
/// `fps_to_refresh_hz` мапит именно в эти значения.
pub const SUPPORTED_REFRESH_RATES: &[u32] = &[23, 24, 25, 30, 50, 60];

#[derive(Debug, Clone)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    pub refresh_hz: u32,
}

impl From<&DEVMODEW> for DisplayMode {
    fn from(dm: &DEVMODEW) -> Self {
        Self {
            width: dm.dmPelsWidth,
            height: dm.dmPelsHeight,
            bpp: dm.dmBitsPerPel,
            refresh_hz: dm.dmDisplayFrequency,
        }
    }
}

/// Текущий режим основного дисплея.
pub fn get_current_mode() -> Option<DisplayMode> {
    unsafe {
        let mut dm: DEVMODEW = mem::zeroed();
        dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
        if EnumDisplaySettingsW(
            PCWSTR::null(),
            ENUM_CURRENT_SETTINGS,
            &mut dm as *mut DEVMODEW,
        )
        .as_bool()
        {
            Some(DisplayMode::from(&dm))
        } else {
            crate::log_info!("[display] EnumDisplaySettingsW failed");
            None
        }
    }
}

/// Ищет ближайший режим с той же шириной/высотой/битностью и частотой,
/// ближайшей к `target_hz`. Возвращает `None`, если режимов с таким
/// разрешением нет вовсе (или WinAPI ругнулся).
pub fn find_best_mode(target_hz: u32) -> Option<DisplayMode> {
    use windows::Win32::Graphics::Gdi::ENUM_DISPLAY_SETTINGS_MODE;

    let current = get_current_mode()?;
    unsafe {
        let mut best: Option<(DisplayMode, u32)> = None;
        let mut i: u32 = 0;
        loop {
            let mut dm: DEVMODEW = mem::zeroed();
            dm.dmSize = mem::size_of::<DEVMODEW>() as u16;

            let ok = EnumDisplaySettingsW(
                PCWSTR::null(),
                ENUM_DISPLAY_SETTINGS_MODE(i),
                &mut dm as *mut DEVMODEW,
            )
            .as_bool();

            if !ok {
                break;
            }

            if dm.dmPelsWidth == current.width
                && dm.dmPelsHeight == current.height
                && dm.dmBitsPerPel == current.bpp
            {
                let diff = (dm.dmDisplayFrequency as i64 - target_hz as i64).unsigned_abs() as u32;
                if best.as_ref().map_or(true, |(_, d)| diff < *d) {
                    best = Some((DisplayMode::from(&dm), diff));
                }
            }
            i += 1;
            if i > 1024 {
                // Защита от драйверов, которые «бесконечно» отдают режимы.
                break;
            }
        }
        best.map(|(m, _)| m)
    }
}

/// Применяет режим через `ChangeDisplaySettingsExW` с `CDS_FULLSCREEN`
/// (режим не сохраняется в реестр и откатывается при выходе/смене).
pub fn apply_mode(mode: &DisplayMode) -> Result<(), String> {
    unsafe {
        let mut dm: DEVMODEW = mem::zeroed();
        dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
        dm.dmPelsWidth = mode.width;
        dm.dmPelsHeight = mode.height;
        dm.dmBitsPerPel = mode.bpp;
        dm.dmDisplayFrequency = mode.refresh_hz;
        // DEVMODE_FIELD_FLAGS реализует BitOr, поэтому OR констант соберёт флаги
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_BITSPERPEL | DM_DISPLAYFREQUENCY;

        let res = ChangeDisplaySettingsExW(
            PCWSTR::null(),
            Some(&dm as *const DEVMODEW),
            None,
            CDS_FULLSCREEN,
            None,
        );
        if res == DISP_CHANGE_SUCCESSFUL {
            crate::log_info!(
                "[display] mode applied: {}x{}@{} (bpp {})",
                mode.width,
                mode.height,
                mode.refresh_hz,
                mode.bpp
            );
            Ok(())
        } else {
            let err = format!("[display] ChangeDisplaySettingsExW failed: {:?}", res);
            crate::log_info!("{}", err);
            Err(err)
        }
    }
}

/// Восстанавливает режим (best-effort, ошибки только логируем).
pub fn restore_mode(original: &DisplayMode) {
    match apply_mode(original) {
        Ok(()) => crate::log_info!(
            "[display] mode restored: {}x{}@{}",
            original.width,
            original.height,
            original.refresh_hz
        ),
        Err(e) => crate::log_info!("[display] failed to restore mode: {}", e),
    }
}

// ---------------------------------------------------------------------------
// RAII-гард
// ---------------------------------------------------------------------------

/// Оригинальный режим, сохранённый при создании первого `DisplayGuard`.
/// Нужен, чтобы `exit_app` мог восстановить частоту, даже если гард
/// не успел дропнуться (например, при `panic = "abort"` в release,
/// или при `app.exit(0)`, который не разворачивает стеки потоков).
static SAVED_MODE: Mutex<Option<DisplayMode>> = Mutex::new(None);

/// RAII-гард: сохраняет текущий режим при создании и восстанавливает при drop.
/// Если гард создан, но `set_refresh_rate` не вызывали — drop просто no-op.
pub struct DisplayGuard;

impl DisplayGuard {
    pub fn new() -> Self {
        if let Ok(mut guard) = SAVED_MODE.lock() {
            if guard.is_none() {
                *guard = get_current_mode();
            }
        }
        Self
    }

    /// Запрашивает переключение на частоту `hz`.
    /// Ошибки только логируются — плеер стартует дальше на текущей частоте.
    pub fn set_refresh_rate(&self, hz: u32) {
        let current = get_current_mode();
        let current_hz = current.as_ref().map(|m| m.refresh_hz).unwrap_or(0);

        let Some(target_mode) = find_best_mode(hz) else {
            crate::log_info!("[display] no mode found for {} Hz, keep current at {} Hz", hz, current_hz);
            return;
        };

        // Пропускаем, только если ТЕКУЩИЙ режим уже совпадает с целевым
        // по всем параметрам (иначе WinAPI дёргаем впустую).
        if let Some(cur) = &current {
            if target_mode.width == cur.width
                && target_mode.height == cur.height
                && target_mode.bpp == cur.bpp
                && target_mode.refresh_hz == cur.refresh_hz
            {
                crate::log_info!("[display] already at {} Hz, skip", hz);
                return;
            }
        }

        crate::log_info!(
            "[display] target {} Hz → applying {} Hz (was {} Hz)",
            hz,
            target_mode.refresh_hz,
            current_hz
        );
        let _ = apply_mode(&target_mode);
    }
}

impl Drop for DisplayGuard {
    fn drop(&mut self) {
        restore_saved_mode();
    }
}

/// Идемпотентно восстанавливает сохранённый режим. Вызывается из `Drop`
/// и из `exit_app`.
pub fn restore_saved_mode() {
    let mode = SAVED_MODE.lock().ok().and_then(|mut g| g.take());
    if let Some(m) = mode {
        restore_mode(&m);
    }
}

// ---------------------------------------------------------------------------
// Определение fps видео (ffprobe)
// ---------------------------------------------------------------------------

/// Запускает `ffprobe` и возвращает fps первого видеопотока, завёрнутый
/// в целевую частоту из `SUPPORTED_REFRESH_RATES`.
///
/// Возвращает `None`, если ffprobe недоступен, вернул ошибку, или fps
/// не распознан. В этом случае гард не трогает частоту.
pub fn determine_fps(file_path: &str) -> Option<u32> {
    let fps = probe_fps(file_path)?;
    let hz = fps_to_refresh_hz(fps)?;
    crate::log_info!("[display] content fps {:.3} → target {} Hz", fps, hz);
    Some(hz)
}

fn probe_fps(file_path: &str) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=r_frame_rate",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            file_path,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        crate::log_info!(
            "[display] ffprobe exited with {} for {}",
            output.status,
            file_path
        );
        return None;
    }

    let s = String::from_utf8_lossy(&output.stdout);
    let line = s.lines().next()?.trim();
    parse_rational(line)
}

fn parse_rational(s: &str) -> Option<f64> {
    if let Some((num, den)) = s.split_once('/') {
        let n: f64 = num.trim().parse().ok()?;
        let d: f64 = den.trim().parse().ok()?;
        if d == 0.0 {
            return None;
        }
        Some(n / d)
    } else {
        s.parse::<f64>().ok()
    }
}

/// Мапит fps контента в целевой Hz для драйвера.
///
/// Особый случай — 23.976: драйверы обычно отдают 23.976-режим как частоту 23,
/// поэтому просим именно 23, а не 24. Для остальных — ближайшее из
/// `SUPPORTED_REFRESH_RATES` (29.97→30, 59.94→60 и т.п.).
fn fps_to_refresh_hz(fps: f64) -> Option<u32> {
    if !fps.is_finite() || fps < 10.0 || fps > 120.0 {
        return None;
    }

    if (fps - 23.976).abs() < 0.01 {
        return Some(23);
    }

    SUPPORTED_REFRESH_RATES
        .iter()
        .copied()
        .min_by_key(|&hz| ((fps - hz as f64).abs() * 1000.0) as u64)
}
