//! 提供商共用的辅助逻辑。

use lyrics_helper_core::{
    ConversionInput, ConversionOptions, FullLyricsResult, InputFile, LyricFormat, RawLyrics,
};

use crate::{
    converter,
    error::{LyricsHelperError, Result},
};

/// 将 TTML 文本解析为 [`FullLyricsResult`]。
///
/// 代码改编自 `providers/amll_ttml_database/mod.rs` 中的
/// `AmllTtmlDatabase::get_full_lyrics`（2026-10-27 抄录），
/// 供两个 AMLL provider 共用，避免重复维护解析流程。
pub(crate) fn parse_ttml_to_full_result(
    ttml_content: String,
    source_name: &str,
) -> Result<FullLyricsResult> {
    let conversion_input = ConversionInput {
        main_lyric: InputFile {
            content: ttml_content.clone(),
            format: LyricFormat::Ttml,
            language: None,
            filename: None,
        },
        translations: vec![],
        romanizations: vec![],
        target_format: LyricFormat::default(),
        user_metadata_overrides: None,
        additional_metadata: None,
    };

    let mut parsed_data =
        converter::parse_and_merge(&conversion_input, &ConversionOptions::default())
            .map_err(|e| LyricsHelperError::Parser(e.to_string()))?;

    parsed_data.source_name = source_name.to_string();

    let raw_lyrics = RawLyrics {
        format: "ttml".to_string(),
        content: ttml_content,
        translation: None,
        romanization: None,
    };

    Ok(FullLyricsResult {
        parsed: parsed_data,
        raw: raw_lyrics,
    })
}
