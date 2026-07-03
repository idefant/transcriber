use crate::settings::{self, EffectiveUiLanguage};

#[derive(Clone, Copy)]
pub enum ConfigErrorText {
    CustomProviderUrlRequired,
    ModelNotAvailableForProvider,
    ModelNotFoundInCatalog,
    ModelNotSelected,
    ProviderApiKeyNotFound,
    ProviderNotFound,
    ProviderNotSelected,
    ProviderUrlNotFound,
    SelectedModelIsNotPostProcessing,
    SelectedModelIsNotSpeechToText,
}

pub fn config_error(app: &tauri::AppHandle, key: ConfigErrorText) -> String {
    config_error_for_language(
        settings::get_effective_ui_language(app).unwrap_or_default(),
        key,
    )
    .to_string()
}

fn config_error_for_language(language: EffectiveUiLanguage, key: ConfigErrorText) -> &'static str {
    match language {
        EffectiveUiLanguage::Ru => match key {
            ConfigErrorText::CustomProviderUrlRequired => "Для Custom-провайдера не задан URL",
            ConfigErrorText::ModelNotAvailableForProvider => {
                "Модель недоступна для этого провайдера"
            }
            ConfigErrorText::ModelNotFoundInCatalog => "Модель не найдена в каталоге",
            ConfigErrorText::ModelNotSelected => "Модель не выбрана",
            ConfigErrorText::ProviderApiKeyNotFound => "API-ключ провайдера не найден",
            ConfigErrorText::ProviderNotFound => "Провайдер не найден",
            ConfigErrorText::ProviderNotSelected => "Провайдер не выбран",
            ConfigErrorText::ProviderUrlNotFound => "URL провайдера не найден",
            ConfigErrorText::SelectedModelIsNotPostProcessing => {
                "Выбранная модель не относится к постобработке"
            }
            ConfigErrorText::SelectedModelIsNotSpeechToText => {
                "Выбранная модель не относится к распознаванию речи"
            }
        },
        EffectiveUiLanguage::En => match key {
            ConfigErrorText::CustomProviderUrlRequired => "URL is required for custom provider",
            ConfigErrorText::ModelNotAvailableForProvider => {
                "Model is not available for this provider"
            }
            ConfigErrorText::ModelNotFoundInCatalog => "Model not found in catalog",
            ConfigErrorText::ModelNotSelected => "Model is not selected",
            ConfigErrorText::ProviderApiKeyNotFound => "Provider API key was not found",
            ConfigErrorText::ProviderNotFound => "Provider was not found",
            ConfigErrorText::ProviderNotSelected => "Provider is not selected",
            ConfigErrorText::ProviderUrlNotFound => "Provider URL was not found",
            ConfigErrorText::SelectedModelIsNotPostProcessing => {
                "Selected model is not a post-processing model"
            }
            ConfigErrorText::SelectedModelIsNotSpeechToText => {
                "Selected model is not a speech-to-text model"
            }
        },
    }
}
