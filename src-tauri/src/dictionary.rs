use crate::{
    error::{AppError, AppResult},
    storage,
};

const DICTIONARY_FILE_NAME: &str = "dictionary.json";

#[tauri::command]
pub fn get_dictionary_words(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    load_dictionary_words(&app).map_err(AppError::into_message)
}

#[tauri::command]
pub fn add_dictionary_word(app: tauri::AppHandle, word: String) -> Result<Vec<String>, String> {
    add_dictionary_word_inner(&app, word).map_err(AppError::into_message)
}

#[tauri::command]
pub fn delete_dictionary_word(app: tauri::AppHandle, word: String) -> Result<Vec<String>, String> {
    delete_dictionary_word_inner(&app, word).map_err(AppError::into_message)
}

fn add_dictionary_word_inner(app: &tauri::AppHandle, word: String) -> AppResult<Vec<String>> {
    let normalized_word = word.trim();

    if normalized_word.is_empty() {
        return load_dictionary_words(app);
    }

    let mut words = load_dictionary_words(app)?;
    let normalized_key = dictionary_word_key(normalized_word);

    if !words
        .iter()
        .any(|word| dictionary_word_key(word) == normalized_key)
    {
        words.push(normalized_word.to_string());
    }

    words = normalize_dictionary_words(words);
    save_dictionary_words(app, &words)?;

    Ok(words)
}

fn delete_dictionary_word_inner(app: &tauri::AppHandle, word: String) -> AppResult<Vec<String>> {
    let normalized_key = dictionary_word_key(&word);
    let mut words = load_dictionary_words(app)?;

    words.retain(|stored_word| dictionary_word_key(stored_word) != normalized_key);
    words = normalize_dictionary_words(words);
    save_dictionary_words(app, &words)?;

    Ok(words)
}

fn load_dictionary_words(app: &tauri::AppHandle) -> AppResult<Vec<String>> {
    let words = storage::load_json_or_default(app, DICTIONARY_FILE_NAME)?;

    Ok(normalize_dictionary_words(words))
}

fn save_dictionary_words(app: &tauri::AppHandle, words: &[String]) -> AppResult<()> {
    storage::save_json(app, DICTIONARY_FILE_NAME, words)
}

fn normalize_dictionary_words(words: Vec<String>) -> Vec<String> {
    let mut normalized_words = Vec::<String>::new();

    for word in words {
        let normalized_word = word.trim();

        if normalized_word.is_empty() {
            continue;
        }

        let normalized_key = dictionary_word_key(normalized_word);

        if normalized_words
            .iter()
            .any(|stored_word| dictionary_word_key(stored_word) == normalized_key)
        {
            continue;
        }

        normalized_words.push(normalized_word.to_string());
    }

    normalized_words.sort_by_key(|word| dictionary_word_key(word));
    normalized_words
}

fn dictionary_word_key(word: &str) -> String {
    word.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dictionary_words() {
        let words = normalize_dictionary_words(vec![
            " Beta ".to_string(),
            "".to_string(),
            "alpha".to_string(),
            "ALPHA".to_string(),
            " beta".to_string(),
            "Gamma".to_string(),
        ]);

        assert_eq!(words, vec!["alpha", "Beta", "Gamma"]);
    }
}
