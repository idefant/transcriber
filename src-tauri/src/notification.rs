//! Нативные системные уведомления Windows об ошибках конфигурации, которые
//! обнаруживаются до старта записи. Клик по уведомлению открывает главное окно
//! на нужной вкладке настроек и закрывает уведомление (штатное поведение toast
//! при активации). На платформах, отличных от Windows, функции — no-op.

#[cfg(windows)]
use std::borrow::Cow;

/// Вкладка настроек, которую открывает клик по уведомлению. Значения совпадают
/// с `SettingsSectionKey` на фронтенде.
#[derive(Clone, Copy)]
pub enum ConfigErrorSection {
    SpeechToText,
    PostProcessing,
}

impl ConfigErrorSection {
    fn section_key(self) -> &'static str {
        match self {
            Self::SpeechToText => "speechToText",
            Self::PostProcessing => "postProcessing",
        }
    }
}

/// Проблема конфигурации, из-за которой диктовку нельзя начать.
pub struct ConfigError {
    pub section: ConfigErrorSection,
    pub message: String,
}

#[cfg(windows)]
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenSettingsPayload {
    section: &'static str,
}

/// Показывает системное уведомление об ошибке конфигурации. По клику открывает
/// главное окно на соответствующей вкладке настроек.
#[cfg(windows)]
pub fn show_config_error(app: &tauri::AppHandle, error: &ConfigError) {
    use tauri::Emitter;
    use tauri_winrt_notification::Toast;

    let language = crate::settings::get_effective_ui_language(app).unwrap_or_default();
    let (title, body) = localized_text(language, error);
    let app_id = toast_app_id(app);

    let handle = app.clone();
    let section = error.section;

    let result = Toast::new(app_id.as_ref())
        .title(&title)
        .text1(&body)
        .on_activated(move |_action| {
            let handle = handle.clone();
            // Операции с окном должны выполняться в главном потоке; сам toast
            // Windows закрывает автоматически при активации.
            let _ = handle.clone().run_on_main_thread(move || {
                let _ = crate::background::show_main_window(&handle);
                let _ = handle.emit(
                    "open-settings",
                    OpenSettingsPayload {
                        section: section.section_key(),
                    },
                );
            });
            Ok(())
        })
        .show();

    // Best-effort: если уведомление показать не удалось, диктовку это ломать
    // не должно, но оставляем след в dev-консоли для диагностики.
    if let Err(error) = result {
        eprintln!("Failed to show config error notification: {error}");
    }
}

#[cfg(not(windows))]
pub fn show_config_error(_app: &tauri::AppHandle, _error: &ConfigError) {}

#[cfg(windows)]
fn toast_app_id(app: &tauri::AppHandle) -> Cow<'static, str> {
    use tauri_winrt_notification::Toast;

    let app_id = app.config().identifier.clone();

    // Для dev-сборки Tauri использует отдельный identifier без установленного
    // AppUserModelID. WinRT toast с таким ID может быть молча подавлен Windows,
    // поэтому используем documented fallback из самой библиотеки.
    if app_id.ends_with(".dev") {
        Cow::Borrowed(Toast::POWERSHELL_APP_ID)
    } else {
        Cow::Owned(app_id)
    }
}

#[cfg(windows)]
fn localized_text(
    language: crate::settings::EffectiveUiLanguage,
    error: &ConfigError,
) -> (String, String) {
    use crate::settings::EffectiveUiLanguage;

    match language {
        EffectiveUiLanguage::Ru => {
            let area = match error.section {
                ConfigErrorSection::SpeechToText => "распознавания речи",
                ConfigErrorSection::PostProcessing => "постобработки",
            };
            (
                "Не удалось начать распознавание".to_string(),
                format!(
                    "Проверьте настройки {area}: {}. Нажмите, чтобы открыть настройки.",
                    error.message
                ),
            )
        }
        EffectiveUiLanguage::En => {
            let area = match error.section {
                ConfigErrorSection::SpeechToText => "speech-to-text",
                ConfigErrorSection::PostProcessing => "post-processing",
            };
            (
                "Couldn't start dictation".to_string(),
                format!(
                    "Check the {area} settings: {}. Click to open settings.",
                    error.message
                ),
            )
        }
    }
}
