import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface PlayerState {
  /** Громкость встроенного аудиоплеера в диапазоне 0..1.5 (до 150%). По умолчанию 1 (100%). Общая для всех записей. */
  volume: number;
  /** Сохраняет громкость плеера; значение переживает смену записи и перезапуск приложения. */
  setVolume: (volume: number) => void;
}

/**
 * Хранит настройки встроенного аудиоплеера истории и сохраняет их в
 * `localStorage`, чтобы выбранная громкость применялась к каждому плееру и
 * не сбрасывалась между записями и перезапусками приложения.
 */
export const usePlayerStore = create<PlayerState>()(
  persist(
    (set) => ({
      volume: 1,

      setVolume: (volume) => {
        set({ volume });
      },
    }),
    { name: 'transcriber-player' },
  ),
);
