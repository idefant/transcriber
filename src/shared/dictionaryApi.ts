import { invoke } from '@tauri-apps/api/core';

export const getDictionaryWords = () => invoke<string[]>('get_dictionary_words');

export const addDictionaryWord = (word: string) =>
  invoke<string[]>('add_dictionary_word', { word });

export const deleteDictionaryWord = (word: string) =>
  invoke<string[]>('delete_dictionary_word', { word });
