use regex::Regex;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct ParsedVideo {
    pub title: String,
    pub year: Option<u32>,
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub codec: Option<String>,
}

pub fn parse_filename(filename: &str) -> ParsedVideo {
    // 1. Убираем расширение
    let stem = filename
        .rsplit_once('.')
        .map(|(name, _ext)| name)
        .unwrap_or(filename);

    // 2. Заменяем разделители на пробелы
    let normalized = stem.replace(['.', '_', '-'], " ");
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    // 3. Ищем год
    let year_re = Regex::new(r"\b(19|20)\d{2}\b").unwrap();
    let year = year_re
        .find(&normalized)
        .and_then(|m| m.as_str().parse::<u32>().ok());

    // 4. Title — всё до года
    let title = if let Some(year_match) = year_re.find(&normalized) {
        normalized[..year_match.start()].trim().to_string()
    } else {
        normalized.clone()
    };

    // Всё после года — метаданные
    let metadata = if let Some(year_match) = year_re.find(&normalized) {
        normalized[year_match.end()..].to_string()
    } else {
        String::new()
    };

    // 5. Resolution
    let resolution_re = Regex::new(r"(?i)\b(2160p|1080p|720p|480p|4K)\b").unwrap();
    let resolution = resolution_re
        .find(&metadata)
        .map(|m| m.as_str().to_string());

    // 6. Source
    let source_re = Regex::new(r"(?i)\b(WEB[- ]?DL|WEBRip|BluRay|BDRip|HDTV|DVDRip)\b").unwrap();
    let source = source_re
        .find(&metadata)
        .map(|m| m.as_str().to_string());

    // 7. Codec
    let codec_re = Regex::new(r"(?i)\b(x265|x264|H\.?265|H\.?264|HEVC|AVC)\b").unwrap();
    let codec = codec_re
        .find(&metadata)
        .map(|m| m.as_str().to_string());

    ParsedVideo {
        title: title.trim().to_string(),
        year,
        resolution,
        source,
        codec,
    }
}