use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use i18n_embed::{fluent::FluentLanguageLoader, unic_langid::LanguageIdentifier, LanguageLoader};
use rust_embed::RustEmbed;

use crate::settings::{self, EffectiveUiLanguage};

const DOMAIN: &str = "backend";

#[derive(RustEmbed)]
#[folder = "i18n"]
struct Localizations;

pub struct BackendI18n {
    loader: FluentLanguageLoader,
    selected_language: Mutex<Option<EffectiveUiLanguage>>,
}

impl BackendI18n {
    pub fn new() -> Self {
        let loader = FluentLanguageLoader::new(DOMAIN, parse_language_identifier("en"));
        loader
            .load_fallback_language(&Localizations)
            .expect("failed to load fallback backend locale");

        Self {
            loader,
            selected_language: Mutex::new(Some(EffectiveUiLanguage::En)),
        }
    }

    fn translate(
        &self,
        language: EffectiveUiLanguage,
        key: &str,
        args: &[(&str, String)],
    ) -> String {
        let Ok(mut selected_language) = self.selected_language.lock() else {
            return key.to_string();
        };

        if selected_language.as_ref() != Some(&language) {
            if self.load_language(language).is_ok() {
                *selected_language = Some(language);
            } else if self.load_language(EffectiveUiLanguage::En).is_ok() {
                *selected_language = Some(EffectiveUiLanguage::En);
            } else {
                return key.to_string();
            }
        }

        if !self.loader.has(key) {
            return key.to_string();
        }

        if args.is_empty() {
            self.loader.get(key)
        } else {
            let mut mapped_args = HashMap::new();

            for (name, value) in args {
                mapped_args.insert(*name, value.as_str());
            }

            self.loader.get_args(key, mapped_args)
        }
    }

    fn load_language(
        &self,
        language: EffectiveUiLanguage,
    ) -> Result<(), i18n_embed::I18nEmbedError> {
        let primary = language_identifier(language);

        if matches!(language, EffectiveUiLanguage::En) {
            self.loader.load_languages(&Localizations, &[primary])
        } else {
            self.loader.load_languages(
                &Localizations,
                &[primary, language_identifier(EffectiveUiLanguage::En)],
            )
        }
    }
}

fn backend_i18n() -> &'static BackendI18n {
    static BACKEND_I18N: OnceLock<BackendI18n> = OnceLock::new();

    BACKEND_I18N.get_or_init(BackendI18n::new)
}

pub fn text(app: &tauri::AppHandle, key: &str) -> String {
    text_with(app, key, &[])
}

pub fn text_with(app: &tauri::AppHandle, key: &str, args: &[(&str, String)]) -> String {
    let language = settings::get_effective_ui_language(app).unwrap_or_default();

    text_for_language(language, key, args)
}

pub fn text_for_language(
    language: EffectiveUiLanguage,
    key: &str,
    args: &[(&str, String)],
) -> String {
    backend_i18n().translate(language, key, args)
}

fn language_identifier(language: EffectiveUiLanguage) -> LanguageIdentifier {
    match language {
        EffectiveUiLanguage::En => parse_language_identifier("en"),
        EffectiveUiLanguage::Ru => parse_language_identifier("ru"),
    }
}

fn parse_language_identifier(value: &str) -> LanguageIdentifier {
    value.parse().expect("valid hard-coded language identifier")
}
