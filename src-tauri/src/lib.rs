mod audio_mute;
mod autostart;
mod background;
mod catalog;
mod debug_log;
mod dictation;
mod dictionary;
mod error;
mod history;
mod keyboard;
mod migrations;
mod overlay;
mod processing;
mod providers;
mod recording;
mod runner;
mod settings;
mod shortcut_hook;
mod storage;
mod updater;

pub fn run() {
    tauri::Builder::default()
        .manage(background::BackgroundRuntime::default())
        .manage(debug_log::DebugLogRuntime::default())
        .manage(dictation::DictationRuntime::default())
        .manage(updater::PendingUpdate::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();

            migrations::run(&app_handle)?;
            overlay::create_recording_overlay(&app_handle)?;
            dictation::register_dictation_shortcut(&app_handle)?;
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
            catalog::get_model_catalog,
            processing::get_processing_config,
            processing::get_default_prompts,
            processing::update_stt_config,
            processing::update_post_process_config,
            runner::run_stt_test,
            runner::run_post_process_test,
            dictation::cancel_dictation,
            dictation::dictation_shortcut_pressed,
            dictation::dictation_shortcut_released,
            shortcut_hook::set_hotkey_capture_active,
            history::get_history_groups,
            history::delete_history_record,
            history::open_history_audio,
            history::repeat_history_record,
            history::repeat_history_transcription,
            history::repeat_history_post_processing,
            updater::check_for_update,
            updater::download_and_install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transcriber");
}
