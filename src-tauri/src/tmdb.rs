use crate::cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct TmdbSearchResponse {
    pub results: Vec<TmdbMovie>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TmdbMovie {
    pub id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub vote_average: Option<f64>,
    pub release_date: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct TmdbTvSearchResponse {
    pub results: Vec<TmdbShow>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TmdbShow {
    pub id: u32,
    pub name: String, // у сериалов "name", не "title"
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub vote_average: Option<f64>,
    pub first_air_date: Option<String>, // у сериалов "first_air_date", не "release_date"
}

#[derive(Serialize, Debug, Clone)]
pub struct TmdbSearchResult {
    pub id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub year: Option<u32>,
}

#[derive(Serialize, Clone)]
pub struct EpisodeMeta {
    pub name: Option<String>,
    pub overview: Option<String>,
    pub still_url: Option<String>,  // полный URL на TMDB (w300)
    pub still_path: Option<String>, // /abc123.jpg — для локального кэша
}

#[derive(Deserialize, Debug)]
struct TmdbVideosResponse {
    results: Vec<TmdbVideo>,
}

#[derive(Deserialize, Debug, Clone)]
struct TmdbVideo {
    key: String,
    site: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    iso_639_1: Option<String>,
    official: Option<bool>,
}

#[tauri::command]
pub async fn get_poster(
    state: tauri::State<'_, cache::AppState>,
    tmdb_id: u32,
    poster_path: String,
) -> Result<String, String> {
    let posters_dir = cache::posters_dir();
    let file_path = posters_dir.join(format!("{}.jpg", tmdb_id));
    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    // Если уже скачан — возвращаем путь
    if file_path.exists() {
        crate::log_info!("  → Poster cache HIT: {}", tmdb_id);
        return Ok(file_path.to_string_lossy().to_string());
    }

    // Иначе качаем
    crate::log_info!("  → Poster cache MISS: {}, downloading...", tmdb_id);
    let url = format!("https://image.tmdb.org/t/p/w500{}", poster_path);

    let client = build_client(proxy_url.as_deref())?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    std::fs::write(&file_path, &bytes).map_err(|e| format!("Write error: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

pub fn build_client(proxy_url: Option<&str>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));

    if let Some(url) = proxy_url {
        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| format!("Некорректный прокси '{}': {}", url, e))?;
        builder = builder.proxy(proxy);
    }

    builder
        .build()
        .map_err(|e| format!("Не удалось создать HTTP-клиент: {}", e))
}

/// Ищет фильм в TMDB по названию. Год используется только для выбора
/// лучшего результата среди нескольких, но не передаётся в API —
/// потому что год в имени файла может отличаться от года в TMDB
/// (региональные премьеры, цифровые релизы и т.д.).
pub async fn search_movie(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbMovie>, String> {
    let url = format!(
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}&language=ru-RU",
        api_key,
        urlencoding::encode(title)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("TMDB returned status: {}", response.status()));
    }

    let data: TmdbSearchResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(pick_best(data.results, year))
}

/// Выбирает лучший результат: если задан год — тот, у кого год ближе
/// к искомому; если год не задан — первый результат (TMDB сортирует
/// по популярности).
fn pick_best(mut results: Vec<TmdbMovie>, year: Option<u32>) -> Option<TmdbMovie> {
    if results.is_empty() {
        return None;
    }

    if let Some(target_year) = year {
        results.sort_by_key(|m| {
            let movie_year = m
                .release_date
                .as_ref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<i32>().ok())
                .unwrap_or(9999);
            (movie_year - target_year as i32).abs()
        });
    }

    results.into_iter().next()
}

pub async fn search_tv(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbShow>, String> {
    let url = format!(
        "https://api.themoviedb.org/3/search/tv?api_key={}&query={}&language=ru-RU",
        api_key,
        urlencoding::encode(title)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("TMDB returned status: {}", response.status()));
    }

    let data: TmdbTvSearchResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(pick_best_show(data.results, year))
}

fn pick_best_show(mut results: Vec<TmdbShow>, year: Option<u32>) -> Option<TmdbShow> {
    if results.is_empty() {
        return None;
    }

    if let Some(target_year) = year {
        results.sort_by_key(|s| {
            let show_year = s
                .first_air_date
                .as_ref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<i32>().ok())
                .unwrap_or(9999);
            (show_year - target_year as i32).abs()
        });
    }

    results.into_iter().next()
}

pub async fn get_or_fetch(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbMovie>, String> {
    // Убрать проверку кэша, оставить только TMDB
    crate::log_info!("  → TMDB lookup: '{}' ({:?})", title, year);
    search_movie(client, api_key, title, year).await
}

pub async fn get_or_fetch_tv(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbShow>, String> {
    crate::log_info!("  → TMDB TV lookup: '{}' ({:?})", title, year);
    search_tv(client, api_key, title, year).await
}

#[tauri::command]
pub async fn search_tmdb_manual(
    state: tauri::State<'_, cache::AppState>,
    query: String,
    media_type: String,
) -> Result<Vec<TmdbSearchResult>, String> {
    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY not set")?;
    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;

    let endpoint = match media_type.as_str() {
        "movie" => "movie",
        "tv_shows" => "tv",
        _ => return Err(format!("Invalid media_type: {}", media_type)),
    };

    let url = format!(
        "https://api.themoviedb.org/3/search/{}?api_key={}&query={}&language=ru-RU",
        endpoint,
        api_key,
        urlencoding::encode(&query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("TMDB status: {}", response.status()));
    }

    let raw: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let results = raw["results"]
        .as_array()
        .ok_or("No results array")?
        .iter()
        .map(|item| {
            let title = item["title"]
                .as_str()
                .or_else(|| item["name"].as_str())
                .unwrap_or("")
                .to_string();

            let original_title = item["original_title"]
                .as_str()
                .or_else(|| item["original_name"].as_str())
                .map(|s| s.to_string());

            let date = item["release_date"]
                .as_str()
                .or_else(|| item["first_air_date"].as_str());

            let year = date
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u32>().ok());

            let poster_url = item["poster_path"]
                .as_str()
                .map(|p| format!("https://image.tmdb.org/t/p/w200{}", p));

            TmdbSearchResult {
                id: item["id"].as_u64().unwrap_or(0) as u32,
                title,
                original_title,
                overview: item["overview"].as_str().map(|s| s.to_string()),
                poster_url,
                year,
            }
        })
        .collect();

    Ok(results)
}

#[derive(Serialize)]
pub struct MatchResult {
    pub tmdb_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub rating: Option<f64>,
}

#[tauri::command]
pub async fn apply_tmdb_match(
    tmdb_id: u32,
    media_type: String,
    uids: Vec<String>,
    state: tauri::State<'_, crate::cache::AppState>,
) -> Result<MatchResult, String> {
    if uids.is_empty() {
        return Err("uids is empty".to_string());
    }

    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY not set")?;
    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;

    let endpoint = match media_type.as_str() {
        "movie" => "movie",
        "tv_shows" => "tv",
        _ => return Err(format!("Invalid media_type: {}", media_type)),
    };

    let url = format!(
        "https://api.themoviedb.org/3/{}/{}?api_key={}&language=ru-RU",
        endpoint, tmdb_id, api_key
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("TMDB status: {}", response.status()));
    }

    let raw: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let title = raw["title"]
        .as_str()
        .or_else(|| raw["name"].as_str())
        .unwrap_or("")
        .to_string();

    let original_title = raw["original_title"]
        .as_str()
        .or_else(|| raw["original_name"].as_str())
        .map(|s| s.to_string());

    let poster_url = raw["poster_path"]
        .as_str()
        .map(|p| format!("https://image.tmdb.org/t/p/w500{}", p));

    let result = MatchResult {
        tmdb_id,
        title,
        original_title,
        overview: raw["overview"].as_str().map(|s| s.to_string()),
        poster_url: poster_url.clone(),
        rating: raw["vote_average"].as_f64(),
    };

    let poster_path = result
        .poster_url
        .as_ref()
        .and_then(|url| url.split("/t/p/w500").nth(1))
        .map(|s| s.to_string());

    let release_date = raw["release_date"]
        .as_str()
        .or_else(|| raw["first_air_date"].as_str());

    // Lock только здесь, после всех await
    let conn = state.conn();
    crate::cache::update_tmdb_for_uids(
        &conn,
        &uids,
        result.tmdb_id,
        &result.title,
        result.original_title.as_deref(),
        result.overview.as_deref(),
        poster_path.as_deref(),
        result.rating,
        release_date,
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[tauri::command]
pub async fn fetch_poster_preview(
    state: tauri::State<'_, cache::AppState>,
    url: String,
) -> Result<String, String> {
    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    use base64::{engine::general_purpose, Engine as _};
    let encoded = general_purpose::STANDARD.encode(&bytes);

    Ok(format!("data:image/jpeg;base64,{}", encoded))
}

#[tauri::command]
pub async fn fetch_episode_meta(
    tmdb_id: u32,
    season: u32,
    episode: u32,
    uid: String,
    state: tauri::State<'_, crate::cache::AppState>,
) -> Result<EpisodeMeta, String> {
    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY not set")?;
    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;

    let url = format!(
        "https://api.themoviedb.org/3/tv/{}/season/{}/episode/{}?api_key={}&language=ru-RU",
        tmdb_id, season, episode, api_key
    );

    crate::log_info!(
        "  → TMDB episode meta: tv={} s{}e{}",
        tmdb_id,
        season,
        episode
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("TMDB status: {}", response.status()));
    }

    let raw: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let name = raw["name"].as_str().map(|s| s.to_string());
    let overview = raw["overview"].as_str().map(|s| s.to_string());
    let still_path = raw["still_path"].as_str().map(|s| s.to_string());
    let still_url = still_path
        .as_ref()
        .map(|p| format!("https://image.tmdb.org/t/p/w300{}", p));

    let meta = EpisodeMeta {
        name: name.clone(),
        overview: overview.clone(),
        still_url,
        still_path: still_path.clone(),
    };

    // Сохраняем в БД
    let conn = state.conn();
    crate::cache::update_episode_meta(
        &conn,
        &uid,
        name.as_deref(),
        overview.as_deref(),
        still_path.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    Ok(meta)
}

#[tauri::command]
pub async fn get_episode_still(
    state: tauri::State<'_, cache::AppState>,
    tmdb_id: u32,
    season: u32,
    episode: u32,
    still_path: String,
) -> Result<String, String> {
    let dir = cache::episode_stills_dir();
    let file_path = dir.join(format!("{}_{}_{}.jpg", tmdb_id, season, episode));

    if file_path.exists() {
        crate::log_info!(
            "  → Episode still cache HIT: {} s{}e{}",
            tmdb_id,
            season,
            episode
        );
        return Ok(file_path.to_string_lossy().to_string());
    }

    crate::log_info!(
        "  → Episode still cache MISS: {} s{}e{}",
        tmdb_id,
        season,
        episode
    );
    let url = format!("https://image.tmdb.org/t/p/w300{}", still_path);

    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    std::fs::write(&file_path, &bytes).map_err(|e| format!("Write error: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Выбирает один трейлер по приоритету:
/// ru → en → любой YouTube.
/// Внутри локали: Trailer > Teaser > прочее; official > не-official.
/// Учитываем только site == "YouTube".
fn pick_trailer(videos: &[TmdbVideo]) -> Option<String> {
    let youtube: Vec<&TmdbVideo> = videos
        .iter()
        .filter(|v| v.site.as_deref() == Some("YouTube"))
        .collect();

    if youtube.is_empty() {
        return None;
    }

    // Ранг: меньше — лучше. Official и Trailer — самые приоритетные.
    let rank = |v: &TmdbVideo| -> (u8, u8) {
        let kind = match v.kind.as_deref() {
            Some("Trailer") => 0u8,
            Some("Teaser") => 1u8,
            _ => 2u8,
        };
        let official = if v.official == Some(true) { 0u8 } else { 1u8 };
        (kind, official)
    };

    let by_lang = |lang: &str| -> Option<&TmdbVideo> {
        youtube
            .iter()
            .copied()
            .filter(|v| v.iso_639_1.as_deref() == Some(lang))
            .min_by_key(|v| rank(v))
    };

    by_lang("ru")
        .or_else(|| by_lang("en"))
        .or_else(|| youtube.iter().copied().min_by_key(|v| rank(v)))
        .map(|v| v.key.clone())
}

async fn fetch_videos_for(
    client: &Client,
    endpoint: &str,
    tmdb_id: u32,
    api_key: &str,
    lang: &str,
) -> Result<Vec<TmdbVideo>, String> {
    let url = format!(
        "https://api.themoviedb.org/3/{}/{}/videos?api_key={}&language={}",
        endpoint, tmdb_id, api_key, lang
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error ({}): {}", lang, e))?;

    if !resp.status().is_success() {
        return Err(format!("TMDB status ({}): {}", lang, resp.status()));
    }

    let data: TmdbVideosResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse error ({}): {}", lang, e))?;

    crate::log_info!("  → TMDB videos [{}]: {} entries", lang, data.results.len());
    Ok(data.results)
}

#[tauri::command]
pub async fn fetch_trailer(
    tmdb_id: u32,
    media_type: String,
    uid: String,
    state: tauri::State<'_, crate::cache::AppState>,
) -> Result<Option<String>, String> {
    // 1. Сначала смотрим кэш в БД — если уже ходили, возвращаем как есть.
    {
        let conn = state.conn();
        if let Some((key, fetched)) = crate::cache::get_trailer_state(&conn, &uid) {
            if fetched {
                crate::log_info!("  → Trailer cache HIT: uid={}, key={:?}", uid, key);
                return Ok(key);
            }
        }
    }

    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY not set")?;
    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;

    let endpoint = match media_type.as_str() {
        "movie" => "movie",
        "tv_shows" => "tv",
        _ => return Err(format!("Invalid media_type: {}", media_type)),
    };

    let _url = format!(
        "https://api.themoviedb.org/3/{}/{}/videos?api_key={}&language=ru-RU",
        endpoint, tmdb_id, api_key
    );

    let ru = fetch_videos_for(&client, endpoint, tmdb_id, &api_key, "ru-RU").await?;
    let en = fetch_videos_for(&client, endpoint, tmdb_id, &api_key, "en-US").await?;

    // Дедуп по key: TMDB иногда возвращает один и тот же ролик в обоих ответах,
    // если он помечен как multi-language.
    let mut seen = std::collections::HashSet::new();
    let mut all: Vec<TmdbVideo> = Vec::with_capacity(ru.len() + en.len());
    for v in ru.into_iter().chain(en.into_iter()) {
        if seen.insert(v.key.clone()) {
            all.push(v);
        }
    }

    crate::log_info!("  → TMDB videos merged: {} entries", all.len());

    let picked = pick_trailer(&all);
    crate::log_info!("  → TMDB trailer picked: {:?}", picked);

    // 2. Пишем результат в БД (даже если None — выставляем fetched = 1).
    {
        let conn = state.conn();
        crate::cache::set_trailer(&conn, &uid, picked.as_deref()).map_err(|e| e.to_string())?;
    }

    Ok(picked)
}

#[tauri::command]
pub async fn get_trailer_thumbnail(
    tmdb_id: u32,
    media_type: String,
    uid: String,
    state: tauri::State<'_, crate::cache::AppState>,
) -> Result<String, String> {
    // 1. Читаем ключ из БД (в сеть за ключом не ходим — его уже сохранил fetch_trailer).
    let trailer_key = {
        let conn = state.conn();
        crate::cache::get_trailer_state(&conn, &uid)
            .and_then(|(key, _)| key)
            .ok_or_else(|| "No trailer key for uid".to_string())?
    };

    let endpoint = match media_type.as_str() {
        "movie" => "movie",
        "tv_shows" => "tv",
        _ => return Err(format!("Invalid media_type: {}", media_type)),
    };

    let dir = crate::cache::trailers_dir();
    let file_path = dir.join(format!("{}_{}.jpg", endpoint, tmdb_id));

    if file_path.exists() {
        crate::log_info!("  → Trailer thumb cache HIT: {}={}", endpoint, tmdb_id);
        return Ok(file_path.to_string_lossy().to_string());
    }

    crate::log_info!(
        "  → Trailer thumb cache MISS: {}={} key={}",
        endpoint,
        tmdb_id,
        trailer_key
    );

    let url = format!("https://img.youtube.com/vi/{}/hqdefault.jpg", trailer_key);

    let proxy_url = {
        let s = state.settings();
        s.proxy_url.clone()
    };
    let client = build_client(proxy_url.as_deref())?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP status: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    std::fs::write(&file_path, &bytes).map_err(|e| format!("Write error: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}
