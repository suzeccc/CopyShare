use std::{collections::HashMap, sync::OnceLock};

use crate::models::{AppConfig, UiLanguage};

const ENGLISH_CATALOG: &str = include_str!("../../locales/en-US.json");
const TRADITIONAL_CATALOG: &str = include_str!("../../locales/zh-TW.json");
const JAPANESE_CATALOG: &str = include_str!("../../locales/ja-JP.json");

fn sorted_catalog(source: &str, fallback: Option<&str>) -> Vec<(String, String)> {
    let mut map: HashMap<String, String> = fallback
        .map(|catalog| serde_json::from_str(catalog).expect("valid fallback locale catalog"))
        .unwrap_or_default();
    map.extend(serde_json::from_str::<HashMap<String, String>>(source).expect("valid locale catalog"));
    let mut entries = map.into_iter().filter(|(key, value)| key != value).collect::<Vec<_>>();
    entries.sort_by(|left, right| right.0.chars().count().cmp(&left.0.chars().count()));
    entries
}

fn catalog_for_language(language: UiLanguage) -> &'static Vec<(String, String)> {
    static ENGLISH: OnceLock<Vec<(String, String)>> = OnceLock::new();
    static TRADITIONAL: OnceLock<Vec<(String, String)>> = OnceLock::new();
    static JAPANESE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    match language {
        UiLanguage::ZhTw => TRADITIONAL.get_or_init(|| sorted_catalog(TRADITIONAL_CATALOG, None)),
        UiLanguage::JaJp => JAPANESE.get_or_init(|| sorted_catalog(JAPANESE_CATALOG, Some(ENGLISH_CATALOG))),
        _ => ENGLISH.get_or_init(|| sorted_catalog(ENGLISH_CATALOG, None)),
    }
}

pub fn effective_language(language: UiLanguage) -> UiLanguage {
    match language {
        UiLanguage::System => {
            let system = sys_locale::get_locale().unwrap_or_default().to_ascii_lowercase();
            if system.starts_with("zh-tw") || system.starts_with("zh-hk") || system.starts_with("zh-mo") {
                UiLanguage::ZhTw
            } else if system.starts_with("zh") {
                UiLanguage::ZhCn
            } else if system.starts_with("ja") {
                UiLanguage::JaJp
            } else {
                UiLanguage::EnUs
            }
        }
        explicit => explicit,
    }
}

pub fn translate(config: &AppConfig, source: &str) -> String {
    translate_for_language(config.ui_language, source)
}

pub fn translate_with_protected(
    config: &AppConfig,
    source: &str,
    protected_values: &[String],
) -> String {
    let mut protected = protected_values
        .iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    protected.sort_by_key(|value| std::cmp::Reverse(value.chars().count()));

    let mut guarded = source.to_string();
    let mut replacements = Vec::new();
    for (index, value) in protected.into_iter().enumerate() {
        let token = format!("\u{e000}{index}\u{e001}");
        if guarded.contains(value.as_str()) {
            guarded = guarded.replace(value.as_str(), &token);
            replacements.push((token, value.as_str()));
        }
    }

    let mut translated = translate(config, &guarded);
    for (token, value) in replacements {
        translated = translated.replace(&token, value);
    }
    translated
}

pub fn translate_for_language(language: UiLanguage, source: &str) -> String {
    if effective_language(language) == UiLanguage::ZhCn {
        return source.to_string();
    }

    let mut translated = source.to_string();
    for (phrase, replacement) in catalog_for_language(effective_language(language)) {
        if translated.contains(phrase) {
            translated = translated.replace(phrase, replacement);
        }
    }
    translated
        .replace('，', ", ")
        .replace('。', ".")
        .replace('：', ": ")
        .replace('、', ", ")
        .replace('；', "; ")
        .replace('？', "?")
        .replace('！', "!")
}

#[cfg(test)]
mod tests {
    use super::{effective_language, translate_for_language, translate_with_protected};
    use crate::models::{AppConfig, UiLanguage};

    #[test]
    fn explicit_language_is_preserved() {
        assert_eq!(effective_language(UiLanguage::ZhCn), UiLanguage::ZhCn);
        assert_eq!(effective_language(UiLanguage::EnUs), UiLanguage::EnUs);
        assert_eq!(effective_language(UiLanguage::ZhTw), UiLanguage::ZhTw);
        assert_eq!(effective_language(UiLanguage::JaJp), UiLanguage::JaJp);
    }

    #[test]
    fn translates_dynamic_native_status_fragments() {
        assert_eq!(
            translate_for_language(
                UiLanguage::EnUs,
                "CopyShare - 运行中，已连接 2 台设备"
            ),
            "CopyShare - Running, Connected 2 devices"
        );
    }

    #[test]
    fn preserves_user_content_inside_localized_native_messages() {
        let mut config = AppConfig::default();
        config.ui_language = UiLanguage::EnUs;
        assert_eq!(
            translate_with_protected(
                &config,
                "来自 我的设置：设置",
                &["我的设置".to_string(), "设置".to_string()],
            ),
            "from 我的设置: 设置"
        );
    }
}
