mod dictionary;
mod error;
mod providers;
mod settings;
mod storage;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            settings::get_app_settings,
            settings::update_app_settings,
            dictionary::get_dictionary_words,
            dictionary::add_dictionary_word,
            dictionary::delete_dictionary_word,
            providers::get_providers,
            providers::create_provider,
            providers::update_provider,
            providers::delete_provider,
            providers::validate_provider_config,
            providers::list_provider_models,
            providers::toggle_favorite_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transcriber");
}
