mod audio_mute;
mod autostart;
mod background;
mod catalog;
mod db;
mod debug_log;
mod dictation;
mod dictionary;
mod error;
mod history;
mod i18n;
mod keyboard;
mod maintenance;
mod media_control;
mod migrations;
mod notification;
mod overlay;
mod processing;
mod providers;
mod recording;
mod runner;
mod settings;
mod shortcut_hook;
mod storage;
mod updater;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Повторный запуск: фокусируем окно уже работающего экземпляра.
            // Уважаем флаг автозапуска --hidden — окно не показываем.
            if !args.iter().any(|arg| arg == autostart::HIDDEN_START_ARG) {
                let _ = background::show_main_window(app);
            }
        }))
        .manage(background::BackgroundRuntime::default())
        .manage(debug_log::DebugLogRuntime::default())
        .manage(dictation::DictationRuntime::default())
        .manage(overlay::OverlayNoticeRuntime::default())
        .manage(updater::PendingUpdate::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // База истории открывается до миграций: миграция схемы v3
            // импортирует в неё history.json.
            db::init(&app_handle)?;

            let startup = migrations::run(&app_handle)?;
            let data_too_new = matches!(startup, migrations::StartupData::TooNew);
            app_handle.manage(maintenance::StartupState { data_too_new });

            if data_too_new {
                // Данные принадлежат более новой версии приложения. Не
                // запускаем диктовку/оверлей и не трогаем данные; показываем
                // главное окно, чтобы фронтенд вывел экран с предложением
                // обновиться или сбросить данные.
                background::setup_background_mode(&app_handle)?;
                return Ok(());
            }

            overlay::create_recording_overlay(&app_handle)?;
            dictation::register_dictation_shortcut(&app_handle)?;
            dictation::prewarm_recorder(&app_handle);
            media_control::prewarm();
            background::setup_background_mode(&app_handle)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_app_settings,
            settings::update_app_settings,
            debug_log::open_debug_logs_folder,
            dictionary::get_dictionary_words,
            dictionary::add_dictionary_word,
            dictionary::delete_dictionary_word,
            providers::get_providers,
            providers::create_provider,
            providers::update_provider,
            providers::delete_provider,
            providers::validate_provider_config,
            providers::list_provider_models,
            providers::list_openrouter_model_providers,
            catalog::get_model_catalog,
            processing::get_processing_config,
            processing::get_default_prompts,
            processing::update_stt_config,
            processing::update_post_process_config,
            runner::run_stt_test,
            runner::run_post_process_test,
            dictation::cancel_dictation,
            dictation::toggle_pause_dictation,
            dictation::dictation_shortcut_pressed,
            dictation::dictation_shortcut_released,
            dictation::copy_latest_history_text,
            dictation::paste_latest_history_text,
            dictation::repeat_latest_history_record,
            overlay::get_overlay_state,
            overlay::dismiss_overlay,
            overlay::overlay_notice_mouse_move,
            overlay::overlay_notice_mouse_leave,
            shortcut_hook::set_hotkey_capture_active,
            history::get_history_groups,
            history::search_history_records,
            history::delete_history_record,
            history::open_history_audio,
            history::open_history_record,
            history::repeat_history_record,
            history::repeat_history_transcription,
            history::repeat_history_post_processing,
            updater::check_for_update,
            updater::download_and_install_update,
            maintenance::get_startup_status,
            maintenance::reset_app_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transcriber");
}
