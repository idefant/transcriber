use std::sync::OnceLock;

use base64::{engine::general_purpose::STANDARD, Engine};
use rustc_hash::FxHashMap;
use tiktoken_rs::CoreBPE;

const WHISPER_PATTERN: &str =
    r"'s|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+";

static WHISPER_TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

/// Возвращает точное число токенов итогового prompt в multilingual-токенизаторе Whisper.
pub fn count_tokens(prompt: &str) -> usize {
    whisper_tokenizer().encode_ordinary(prompt).len()
}

fn whisper_tokenizer() -> &'static CoreBPE {
    WHISPER_TOKENIZER.get_or_init(|| {
        let ranks = include_str!("../assets/multilingual.tiktoken")
            .lines()
            .map(|line| {
                let (token, rank) = line
                    .rsplit_once(' ')
                    .expect("строка словаря Whisper должна содержать токен и ранг");
                // Последняя запись официального словаря — `= 50256`. Python-реализация
                // Whisper декодирует этот нетипичный Base64-маркер как пустую последовательность.
                let token = if token == "=" {
                    Vec::new()
                } else {
                    STANDARD
                        .decode(token)
                        .expect("словарь Whisper должен содержать Base64-токены")
                };
                let rank = rank
                    .parse()
                    .expect("словарь Whisper должен содержать числовые ранги");

                (token, rank)
            })
            .collect::<FxHashMap<_, _>>();

        CoreBPE::new(ranks, FxHashMap::default(), WHISPER_PATTERN)
            .expect("регулярное выражение токенизатора Whisper должно быть корректным")
    })
}
