use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct TvParseResult {
    pub title: String,
    pub year: Option<u32>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
}

/// Парсит имя файла. Сезон/эпизод берутся из файла, fallback на parent_dirs
/// (в порядке от ближайшей папки к дальней).
///
/// `parent_dirs` — имена папок от родительской к прародительской. Например,
/// для `Silo/Silo S01/SiloE1.mkv` это будет `["Silo S01", "Silo"]`.
pub fn parse_tv_filename(filename: &str, parent_dirs: &[&str]) -> TvParseResult {
    let mut result = parse_from_filename(filename);

    // Если сезон не найден в имени файла — ищем в родительских папках.
    if result.season.is_none() {
        for dir in parent_dirs {
            if let Some(season) = extract_season_from_dir(dir) {
                result.season = Some(season);
                break;
            }
        }
    }

    // Эпизод только из имени файла. Если там его нет — не угадываем.
    // (по решению LUMI-22: файлы без эпизода пользователь правит руками)

    result
}

fn parse_from_filename(filename: &str) -> TvParseResult {
    let stem = filename
        .rsplit_once('.')
        .map(|(name, _ext)| name)
        .unwrap_or(filename);

    let normalized = stem.replace(['.', '_'], " ");

    // SxxExx
    let se_re = Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})\b").unwrap();
    if let Some(caps) = se_re.captures(&normalized) {
        let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let title_end = caps.get(0).unwrap().start();
        let title = clean_title(&normalized[..title_end]);
        let year = extract_year(&normalized);
        return TvParseResult { title, year, season, episode };
    }

    // 1x01
    let alt_re = Regex::new(r"\b(\d{1,2})x(\d{2,3})\b").unwrap();
    if let Some(caps) = alt_re.captures(&normalized) {
        let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let title_end = caps.get(0).unwrap().start();
        let title = clean_title(&normalized[..title_end]);
        let year = extract_year(&normalized);
        return TvParseResult { title, year, season, episode };
    }

    // Только сезон SN (без E)
    let s_re = Regex::new(r"(?i)\bS(\d{1,2})\b").unwrap();
    if let Some(caps) = s_re.captures(&normalized) {
        let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let title_end = caps.get(0).unwrap().start();
        let title = clean_title(&normalized[..title_end]);
        let year = extract_year(&normalized);
        return TvParseResult { title, year, season, episode: None };
    }

    // Не распознали — возвращаем как есть
    TvParseResult {
        title: clean_title(&normalized),
        year: extract_year(&normalized),
        season: None,
        episode: None,
    }
}

/// Извлекает номер сезона из имени папки.
/// Поддерживает: Season N, Season.N, Сезон N, N сезон, SN, SNN.
fn extract_season_from_dir(dir_name: &str) -> Option<u32> {
    let normalized = dir_name.replace(['.', '_'], " ");

    // (Season N) / Season N — с числом после слова
    let season_word_re = Regex::new(
        r"(?i)\b(?:season|сезон)\s*[\(\s]?\s*(\d{1,2})\b"
    ).unwrap();
    if let Some(caps) = season_word_re.captures(&normalized) {
        if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
            return Some(n);
        }
    }

    // N сезон / N сезон: ...
    let n_season_re = Regex::new(r"(?i)\b(\d{1,2})\s*[-й]?\s*сезон\b").unwrap();
    if let Some(caps) = n_season_re.captures(&normalized) {
        if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
            return Some(n);
        }
    }

    // SN / SNN на границе слова (например "Silo S03" или "Silo S1")
    let s_re = Regex::new(r"(?i)\bS(\d{1,2})\b").unwrap();
    if let Some(caps) = s_re.captures(&normalized) {
        if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse().ok()) {
            return Some(n);
        }
    }

    None
}

fn extract_year(s: &str) -> Option<u32> {
    let re = Regex::new(r"\b(19|20)\d{2}\b").unwrap();
    re.find(s).and_then(|m| m.as_str().parse().ok())
}

fn clean_title(s: &str) -> String {
    s.replace(['.', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}