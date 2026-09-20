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

#[tauri::command]
pub async fn get_poster(tmdb_id: u32, poster_path: String) -> Result<String, String> {
    let posters_dir = cache::posters_dir();
    let file_path = posters_dir.join(format!("{}.jpg", tmdb_id));

    // Если уже скачан — возвращаем путь
    if file_path.exists() {
        println!("  → Poster cache HIT: {}", tmdb_id);
        return Ok(file_path.to_string_lossy().to_string());
    }

    // Иначе качаем
    println!("  → Poster cache MISS: {}, downloading...", tmdb_id);
    let url = format!("https://image.tmdb.org/t/p/w500{}", poster_path);

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

    std::fs::write(&file_path, &bytes).map_err(|e| format!("Write error: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
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
                original_title: cached.original_title,
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

pub async fn get_or_fetch_tv(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbShow>, String> {
    // 1. Проверяем кэш
    {
        let conn = cache::init_db().map_err(|e| e.to_string())?;
        if let Some(cached) = cache::find_cached_tv(&conn, title) {
            println!("  → TV Cache HIT: {}", cached.name);
            return Ok(Some(TmdbShow {
                id: cached.tmdb_id,
                name: cached.name,
                original_name: cached.original_name,
                overview: cached.overview,
                poster_path: cached.poster_path,
                vote_average: cached.rating,
                first_air_date: cached.first_air_date,
            }));
        }
    }

    // 2. Идём в TMDB
    println!(
        "  → TV Cache MISS, fetching from TMDB: '{}' ({:?})",
        title, year
    );
    let result = search_tv(client, api_key, title, year).await?;

    // 3. Сохраняем
    if let Some(ref show) = result {
        if let Ok(conn) = cache::init_db() {
            let _ = cache::save_tv_show(&conn, show);
        }
    } else {
        println!("  → No TV results for '{}'", title);
    }

    Ok(result)
}

#[tauri::command]
pub async fn search_tmdb_manual(
    query: String,
    media_type: String,
) -> Result<Vec<TmdbSearchResult>, String> {
    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY not set")?;
    let client = build_client().map_err(|e| e.to_string())?;

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
pub async fn apply_tmdb_match(tmdb_id: u32, media_type: String) -> Result<MatchResult, String> {
    let api_key = std::env::var("TMDB_API_KEY").map_err(|_| "TMDB_API_KEY not set")?;
    let client = build_client().map_err(|e| e.to_string())?;

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
        poster_url,
        rating: raw["vote_average"].as_f64(),
    };

    // Сохраняем в кэш
    cache::save_manual_match(&result, media_type.as_str())?;

    Ok(result)
}

#[tauri::command]
pub async fn fetch_poster_preview(url: String) -> Result<String, String> {
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