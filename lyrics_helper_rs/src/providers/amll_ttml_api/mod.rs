//! 此模块实现了与 AMLL TTML API（<https://api.amll.dev>）交互的 `Provider`。
//!
//! 与 `amll_ttml_database` 的区别：后者下载 GitHub 上的完整索引并在本地匹配，
//! 本实现直接调用服务端 `/v1/lyrics/search` 与 `/v1/lyrics/get`，无需本地索引。

use std::sync::Arc;

use async_trait::async_trait;
use lyrics_helper_core::{
    CoverSize, FullLyricsResult, MatchType, ParsedSourceData, SearchResult, Track, model::generic,
};

use crate::{
    config::AmllApiMirror,
    error::{LyricsHelperError, Result},
    http::HttpClient,
    providers::{
        Provider,
        amll_ttml_api::types::{GetResponse, SearchResponse, SongItem},
        common::parse_ttml_to_full_result,
    },
};

mod types;

const USER_AGENT: &str = "lyrics-helper-rs/0.1.0";
const SEARCH_PAGE_SIZE: u32 = 50;

pub struct AmllTtmlApi {
    http_client: Arc<dyn HttpClient>,
    base_url: String,
}

impl AmllTtmlApi {
    fn endpoint(&self, path: &str) -> String {
        format!("{}{path}", self.base_url.trim_end_matches('/'))
    }

    fn check_status(status: u16, context: &str, song_id: Option<&str>) -> Result<()> {
        match status {
            200 => Ok(()),
            400 => Err(LyricsHelperError::ApiError(format!(
                "{context}: 请求参数无效"
            ))),
            404 => Err(match song_id {
                Some(id) => LyricsHelperError::ApiError(format!("{context}: 未找到 {id}")),
                None => LyricsHelperError::LyricNotFound,
            }),
            429 => Err(LyricsHelperError::RateLimited(format!("{context}: 触发限流"))),
            other => Err(LyricsHelperError::ApiError(format!(
                "{context}: 状态码 {other}"
            ))),
        }
    }
}

#[async_trait]
impl Provider for AmllTtmlApi {
    fn name(&self) -> &'static str {
        "amll-ttml-api"
    }

    async fn with_http_client(http_client: Arc<dyn HttpClient>) -> Result<Self>
    where
        Self: Sized,
    {
        let config = crate::config::load_amll_config().unwrap_or_else(|e| {
            tracing::error!("[AMLL API] 加载配置失败: {e}. 使用官方源。");
            crate::config::AmllConfig::default()
        });

        let base_url = match &config.api_mirror {
            AmllApiMirror::Official => "https://api.amll.dev".to_string(),
            AmllApiMirror::Custom { base_url } => base_url.trim_end_matches('/').to_string(),
        };

        tracing::info!("[AMLL API] 使用 API 根地址: {base_url}");
        Ok(Self {
            http_client,
            base_url,
        })
    }

    async fn search_songs(&self, track: &Track<'_>) -> Result<Vec<SearchResult>> {
        let Some(title) = track.title else {
            return Ok(vec![]);
        };
        if title.trim().is_empty() {
            return Ok(vec![]);
        }

        let artists = track.artists.unwrap_or(&[]).join(" ");
        let page_size = SEARCH_PAGE_SIZE.to_string();
        let mut params: Vec<(&str, &str)> = vec![("musicName", title), ("pageSize", &page_size)];
        if !artists.is_empty() {
            params.push(("artistName", &artists));
        }

        let url = self.endpoint("/v1/lyrics/search");
        let response = self
            .http_client
            .get_with_params_and_headers(&url, &params, &[("User-Agent", USER_AGENT)])
            .await?;

        Self::check_status(response.status, "AMLL API 搜索", None)?;

        let body = response.text()?;
        let parsed: SearchResponse = serde_json::from_str(&body)
            .map_err(|e| LyricsHelperError::Parser(format!("AMLL API 搜索响应解析失败: {e}")))?;

        Ok(parsed
            .data
            .items
            .into_iter()
            .map(|item| into_search_result(&item, self.name()))
            .collect())
    }

    async fn get_full_lyrics(&self, song_id: &str) -> Result<FullLyricsResult> {
        let url = self.endpoint("/v1/lyrics/get");
        let response = self
            .http_client
            .get_with_params_and_headers(&url, &[("filename", song_id)], &[("User-Agent", USER_AGENT)])
            .await?;

        Self::check_status(response.status, "AMLL API 获取歌词", Some(song_id))?;

        let body = response.text()?;
        let parsed: GetResponse = serde_json::from_str(&body)
            .map_err(|e| LyricsHelperError::Parser(format!("AMLL API 歌词响应解析失败: {e}")))?;

        let lyrics = parsed.data.lyrics.ok_or_else(|| {
            LyricsHelperError::Parser("AMLL API 未返回歌词内容".to_string())
        })?;

        parse_ttml_to_full_result(lyrics, self.name())
    }

    async fn get_lyrics(&self, song_id: &str) -> Result<ParsedSourceData> {
        Ok(self.get_full_lyrics(song_id).await?.parsed)
    }

    async fn get_album_info(&self, _: &str) -> Result<generic::Album> {
        Err(LyricsHelperError::ProviderNotSupported(
            "amll-ttml-api 不支持 get_album_info".into(),
        ))
    }

    async fn get_album_songs(
        &self,
        _album_id: &str,
        _page: u32,
        _page_size: u32,
    ) -> Result<Vec<generic::Song>> {
        Err(LyricsHelperError::ProviderNotSupported(
            "amll-ttml-api 不支持 get_album_songs".to_string(),
        ))
    }

    async fn get_singer_songs(
        &self,
        _singer_id: &str,
        _page: u32,
        _page_size: u32,
    ) -> Result<Vec<generic::Song>> {
        Err(LyricsHelperError::ProviderNotSupported(
            "amll-ttml-api 不支持 get_singer_songs".to_string(),
        ))
    }

    async fn get_playlist(&self, _playlist_id: &str) -> Result<generic::Playlist> {
        Err(LyricsHelperError::ProviderNotSupported(
            "amll-ttml-api 不支持 get_playlist".to_string(),
        ))
    }

    async fn get_song_info(&self, _song_id: &str) -> Result<generic::Song> {
        Err(LyricsHelperError::ProviderNotSupported(
            "amll-ttml-api 不支持 get_song_info".to_string(),
        ))
    }

    async fn get_album_cover_url(&self, _album_id: &str, _size: CoverSize) -> Result<String> {
        Err(LyricsHelperError::ProviderNotSupported(
            "amll-ttml-api 不支持 get_album_cover_url".to_string(),
        ))
    }
}

/// 将 API 返回的 `SongItem` 转为统一的 `SearchResult`。
///
/// 转换方式参考 `providers/amll_ttml_database/mod.rs` 的 `into_search_result`
/// （2026-10-27 抄录），差异在于索引条目换成 API 响应。
fn into_search_result(item: &SongItem, provider_name: &str) -> SearchResult {
    let title = item
        .music_names
        .first()
        .cloned()
        .unwrap_or_else(|| item.filename.clone());

    SearchResult {
        provider_id: item.filename.clone(),
        title,
        artists: item
            .artist_names
            .iter()
            .map(|name| generic::Artist {
                id: String::new(),
                name: name.clone(),
            })
            .collect(),
        album: item.album_names.first().cloned(),
        provider_name: provider_name.to_string(),
        match_type: MatchType::default(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> SongItem {
        SongItem {
            id: 269_710_089_745_311,
            filename: "1768754400682-250306205-r6IrpmBd.ttml".to_string(),
            music_names: vec!["ME!".to_string(), "ME! (feat. Brendon Urie)".to_string()],
            artist_names: vec!["Taylor Swift".to_string()],
            album_names: vec!["Lover".to_string()],
        }
    }

    #[test]
    fn search_result_uses_filename_as_provider_id() {
        let result = into_search_result(&item(), "amll-ttml-api");
        assert_eq!(result.provider_id, "1768754400682-250306205-r6IrpmBd.ttml");
        assert_eq!(result.provider_name, "amll-ttml-api");
    }

    #[test]
    fn search_result_takes_first_music_name() {
        let result = into_search_result(&item(), "amll-ttml-api");
        assert_eq!(result.title, "ME!");
        assert_eq!(result.album.as_deref(), Some("Lover"));
        assert_eq!(result.artists.len(), 1);
        assert_eq!(result.artists[0].name, "Taylor Swift");
    }

    #[test]
    fn search_result_falls_back_to_filename_without_music_name() {
        let mut empty = item();
        empty.music_names.clear();
        let result = into_search_result(&empty, "amll-ttml-api");
        assert_eq!(result.title, empty.filename);
    }

    #[test]
    fn status_mapping_covers_documented_codes() {
        assert!(AmllTtmlApi::check_status(200, "ctx", None).is_ok());
        assert!(matches!(
            AmllTtmlApi::check_status(400, "ctx", None),
            Err(LyricsHelperError::ApiError(_))
        ));
        assert!(matches!(
            AmllTtmlApi::check_status(404, "ctx", None),
            Err(LyricsHelperError::LyricNotFound)
        ));
        assert!(matches!(
            AmllTtmlApi::check_status(429, "ctx", None),
            Err(LyricsHelperError::RateLimited(_))
        ));
        assert!(matches!(
            AmllTtmlApi::check_status(502, "ctx", None),
            Err(LyricsHelperError::ApiError(_))
        ));
    }
}
