mod catalog;
mod dictionary;
mod error;
mod processing;
mod providers;
mod runner;
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
            catalog::get_model_catalog,
            processing::get_processing_config,
            processing::get_default_prompts,
            processing::update_stt_config,
            processing::update_post_process_config,
            runner::run_stt_test,
            runner::run_post_process_test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transcriber");
}
