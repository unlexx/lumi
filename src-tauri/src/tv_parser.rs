use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct TvParseResult {
    pub title: String,
    pub year: Option<u32>,
    pub season: Option<u32>,
    pub episode: Option<u32>,
}

pub fn parse_tv_filename(filename: &str) -> TvParseResult {
    // Убираем расширение
    let stem = filename
        .rsplit_once('.')
        .map(|(name, _ext)| name)
        .unwrap_or(filename);

    // Нормализуем: точки/подчёркивания → пробелы
    let normalized = stem.replace(['.', '_'], " ");

    // Ищем SxxExx (основной формат)
    let se_re = Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})\b").unwrap();
    if let Some(caps) = se_re.captures(&normalized) {
        let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let title_end = caps.get(0).unwrap().start();
        let title = clean_title(&normalized[..title_end]);
        let year = extract_year(&normalized);
        return TvParseResult { title, year, season, episode };
    }

    // Формат 1x01
    let alt_re = Regex::new(r"\b(\d{1,2})x(\d{2,3})\b").unwrap();
    if let Some(caps) = alt_re.captures(&normalized) {
        let season = caps.get(1).and_then(|m| m.as_str().parse().ok());
        let episode = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let title_end = caps.get(0).unwrap().start();
        let title = clean_title(&normalized[..title_end]);
        let year = extract_year(&normalized);
        return TvParseResult { title, year, season, episode };
    }

    // Только сезон (папка вроде "House.of.the.Dragon.S03.2160p...")
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