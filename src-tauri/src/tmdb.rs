use crate::cache;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TmdbSearchResponse {
    pub results: Vec<TmdbMovie>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TmdbMovie {
    pub id: u32,
    pub title: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub vote_average: Option<f64>,
    pub release_date: Option<String>,
}

pub fn build_client() -> Result<Client, reqwest::Error> {
    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:9090")?;

    Client::builder()
        .proxy(proxy)
        .user_agent("Lumi/0.1.0")
        .build()
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
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}",
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

/// Главная точка входа: сначала проверяет кэш, потом идёт в TMDB,
/// результат сохраняет в кэш.
pub async fn get_or_fetch(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbMovie>, String> {
    // 1. Проверяем кэш
    {
        let conn = cache::init_db().map_err(|e| e.to_string())?;
        if let Some(cached) = cache::find_cached(&conn, title) {
            println!("  → Cache HIT: {}", cached.title);
            return Ok(Some(TmdbMovie {
                id: cached.tmdb_id,
                title: cached.title,
                overview: cached.overview,
                poster_path: cached.poster_path,
                vote_average: cached.rating,
                release_date: cached.release_date,
            }));
        }
    }

    // 2. Идём в TMDB
    println!(
        "  → Cache MISS, fetching from TMDB: '{}' ({:?})",
        title, year
    );
    let result = search_movie(client, api_key, title, year).await?;

    // 3. Сохраняем в кэш
    if let Some(ref movie) = result {
        if let Ok(conn) = cache::init_db() {
            let _ = cache::save_movie(&conn, movie);
        }
    } else {
        println!("  → No results for '{}'", title);
    }

    Ok(result)
}

/// Скачивает постер через прокси и возвращает как data URL (base64).
/// Временное решение — в LUMI-5 заменим на локальный кэш файлов.
#[tauri::command]
pub async fn fetch_poster(url: String) -> Result<String, String> {
    let client = build_client().map_err(|e| e.to_string())?;

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