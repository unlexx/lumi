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

pub async fn search_movie(
    client: &Client,
    api_key: &str,
    title: &str,
    year: Option<u32>,
) -> Result<Option<TmdbMovie>, String> {
    let mut url = format!(
        "https://api.themoviedb.org/3/search/movie?api_key={}&query={}",
        api_key,
        urlencoding::encode(title)
    );

    if let Some(y) = year {
        url.push_str(&format!("&year={}", y));
    }

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

    Ok(data.results.into_iter().next())
}

#[tauri::command]
pub async fn fetch_poster(url: String) -> Result<String, String> {
    let client = build_client().map_err(|e| e.to_string())?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;
    
    use base64::{engine::general_purpose, Engine as _};
    let encoded = general_purpose::STANDARD.encode(&bytes);
    
    Ok(format!("data:image/jpeg;base64,{}", encoded))
}