export interface ParsedVideo {
  title: string
  year: number | null
  resolution: string | null
  source: string | null
  codec: string | null
}

export interface TmdbInfo {
  id: number
  title: string
  original_title: string | null
  overview: string | null
  poster_url: string | null
  poster_local: string | null
  rating: number | null
}

export interface VideoFile {
  uid: string
  path: string
  name: string
  extension: string
  parsed: ParsedVideo
  tmdb: TmdbInfo | null
  media_type: 'movie' | 'tv_shows'
  watched: boolean
  position: number | null
  duration: number | null
}

export interface Episode {
  number: number
  path: string
  name: string
  parsed: ParsedVideo
  watched: boolean
  position: number | null
  duration: number | null
}

export interface Season {
  number: number
  episodes: Episode[]
}

export interface TvShow {
  title: string
  year: number | null
  seasons: Season[]
  tmdb: TmdbInfo | null
}

export interface Library {
  movies: VideoFile[]
  tv_shows: TvShow[]
}

export interface ContinueItem {
  path: string
  title: string
  poster_url: string | null
  position: number
  duration: number
  progress: number
  media_type: 'movie' | 'tv_shows'
}

export interface TmdbSearchResult {
  id: number
  title: string
  original_title: string | null
  overview: string | null
  poster_url: string | null
  poster_data: string | null
  year: number | null
}

export interface MatchResult {
  tmdb_id: number
  title: string
  original_title: string | null
  overview: string | null
  poster_url: string | null
  rating: number | null
}

export interface UndefinedItem {
  uid: string
  path: string
  file_name: string
  display_title: string
  media_type: 'movie' | 'tv_shows'
  year: number | null
}