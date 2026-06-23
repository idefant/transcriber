import { create } from 'zustand';

import * as dictionaryApi from '#/shared/dictionaryApi';

interface DictionaryState {
  words: string[];
  isLoading: boolean;
  hasLoaded: boolean;
  load: () => Promise<void>;
  addWord: (word: string) => Promise<void>;
  removeWord: (word: string) => Promise<void>;
}

export const useDictionaryStore = create<DictionaryState>((set, get) => ({
  words: [],
  isLoading: false,
  hasLoaded: false,

  load: async () => {
    if (get().isLoading) return;
    set({ isLoading: true });
    try {
      const words = await dictionaryApi.getDictionaryWords();
      set({ words, hasLoaded: true });
    } finally {
      set({ isLoading: false });
    }
  },

  addWord: async (word) => {
    const words = await dictionaryApi.addDictionaryWord(word);
    set({ words });
  },

  removeWord: async (word) => {
    const words = await dictionaryApi.deleteDictionaryWord(word);
    set({ words });
  },
}));
