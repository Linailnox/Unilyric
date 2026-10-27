//! AMLL TTML API 的响应类型。
//!
//! 字段定义对应 <https://amll.dev/api/ttml/openapi.yaml> 的
//! `SongItem` / `SearchResponse` / `GetResponse`，只解析本项目需要的部分。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub status: i32,
    pub data: SearchData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchData {
    pub items: Vec<SongItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongItem {
    #[serde(default)]
    pub id: i64,
    pub filename: String,
    pub music_names: Vec<String>,
    pub artist_names: Vec<String>,
    pub album_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetResponse {
    pub status: i32,
    pub data: SongDetail,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongDetail {
    pub filename: String,
    pub music_names: Vec<String>,
    pub artist_names: Vec<String>,
    pub album_names: Vec<String>,
    pub lyrics: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub status: i32,
    pub error: String,
    pub message: String,
}
