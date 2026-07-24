import { useEffect, useRef, useState } from 'react';
import { clamp } from 'lodash-es';

import { getHistoryAudioData } from '#/shared/historyApi';

import type { HistoryRecord } from '#/models/History';
import { usePlayerStore } from '#/stores';

/** Максимальная громкость плеера: 1.5 соответствует усилению до 150%. */
export const MAX_VOLUME = 1.5;

// Один общий AudioContext на всё приложение: у браузера жёсткий лимит на число
// живых контекстов, поэтому переиспользуем единственный вместо создания на каждый
// плеер.
let sharedAudioContext: AudioContext | undefined;

const getAudioContext = (): AudioContext => {
  sharedAudioContext ??= new AudioContext();
  return sharedAudioContext;
};

// Кэш аудиографа по элементу. createMediaElementSource нельзя вызывать на одном
// и том же элементе повторно (в том числе при двойном монтировании React
// StrictMode), поэтому source-нода создаётся ровно один раз на элемент.
const audioGraphs = new WeakMap<
  HTMLAudioElement,
  { gain: GainNode; source: MediaElementAudioSourceNode }
>();

const getAudioGraph = (audio: HTMLAudioElement) => {
  let graph = audioGraphs.get(audio);
  if (!graph) {
    const context = getAudioContext();
    const source = context.createMediaElementSource(audio);
    const gain = context.createGain();
    source.connect(gain);
    graph = { gain, source };
    audioGraphs.set(audio, graph);
  }
  return graph;
};

interface UseAudioPlayerResult {
  /** Ref, который нужно повесить на скрытый элемент `<audio>`. */
  audioRef: React.RefObject<HTMLAudioElement | null>;
  /** Длительность аудио в секундах: из метаданных `<audio>`, иначе из данных записи. */
  duration: number;
  /** Текущая позиция воспроизведения в секундах. */
  currentTime: number;
  /** Идёт ли воспроизведение прямо сейчас. */
  isPlaying: boolean;
  /** Не удалось загрузить, прочитать или декодировать аудиофайл — плеер недоступен. */
  hasError: boolean;
  /** Громкость в диапазоне 0..{@link MAX_VOLUME}, общая для всех плееров. */
  volume: number;
  /** Переключает воспроизведение между паузой и продолжением. */
  togglePlay: () => void;
  /** Перематывает воспроизведение на указанную позицию в секундах. Вызывается на каждом шаге перетаскивания ползунка. */
  seek: (time: number) => void;
  /** Завершает перетаскивание ползунка перемотки и возвращает синхронизацию позиции с воспроизведением. */
  finishSeek: () => void;
  /** Меняет громкость плеера и запоминает её между записями и перезапусками. */
  changeVolume: (volume: number) => void;
}

/**
 * Управляет встроенным аудиоплеером панели деталей истории: загружает байты
 * аудиофайла записи, отдаёт их скрытому `<audio>` через Blob URL, стартует
 * воспроизведение при монтировании и синхронизирует состояние с событиями
 * элемента. Громкость проходит через Web Audio `GainNode`, что позволяет
 * усиливать её выше 100% (нативный `audio.volume` ограничен единицей). Blob URL
 * и слушатели освобождаются при размонтировании и смене записи. При недоступном
 * или недекодируемом файле переводит плеер в состояние ошибки.
 */
export const useAudioPlayer = (record: HistoryRecord): UseAudioPlayerResult => {
  const audioRef = useRef<HTMLAudioElement>(null);
  const volume = usePlayerStore((state) => state.volume);
  const setVolume = usePlayerStore((state) => state.setVolume);

  // Пока идёт перетаскивание ползунка перемотки, событие `timeupdate` не должно
  // двигать позицию: иначе ручка дёргается между шагом перетаскивания и
  // фактическим воспроизведением.
  const isSeekingRef = useRef(false);

  // Fallback-длительность из данных записи, пока не пришли метаданные <audio>.
  const fallbackDuration = record.audio.durationMs / 1000;
  const [duration, setDuration] = useState(fallbackDuration);
  const [currentTime, setCurrentTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    let objectUrl: string | undefined;
    let isCancelled = false;
    const audio = audioRef.current;

    if (!audio) {
      return;
    }

    const context = getAudioContext();
    const graph = getAudioGraph(audio);
    graph.gain.connect(context.destination);
    // Громкость регулируется через GainNode (её начальное и последующие значения
    // выставляет отдельный эффект по `volume`), поэтому у самого элемента она
    // остаётся максимальной.
    audio.volume = 1;

    const handleTimeUpdate = () => {
      if (!isSeekingRef.current) {
        setCurrentTime(audio.currentTime);
      }
    };
    const handleLoadedMetadata = () => {
      if (Number.isFinite(audio.duration) && audio.duration > 0) {
        setDuration(audio.duration);
      }
    };
    const handlePlay = () => {
      setIsPlaying(true);
    };
    const handlePause = () => {
      setIsPlaying(false);
    };
    const handleEnded = () => {
      setIsPlaying(false);
    };
    const handleError = () => {
      setHasError(true);
    };

    audio.addEventListener('timeupdate', handleTimeUpdate);
    audio.addEventListener('loadedmetadata', handleLoadedMetadata);
    audio.addEventListener('play', handlePlay);
    audio.addEventListener('pause', handlePause);
    audio.addEventListener('ended', handleEnded);
    audio.addEventListener('error', handleError);

    const load = async () => {
      try {
        const buffer = await getHistoryAudioData(record.id);
        if (isCancelled) {
          return;
        }

        const blob = new Blob([buffer], { type: 'audio/wav' });
        objectUrl = URL.createObjectURL(blob);
        audio.src = objectUrl;
        // Autoplay при монтировании плеера. Контекст мог быть создан до жеста
        // пользователя, поэтому возобновляем его перед стартом.
        void context.resume();
        // Игнорируем отказ политики autoplay: пользователь продолжит вручную.
        void audio.play().catch(() => {});
      } catch {
        if (!isCancelled) {
          setHasError(true);
        }
      }
    };

    void load();

    return () => {
      isCancelled = true;
      audio.removeEventListener('timeupdate', handleTimeUpdate);
      audio.removeEventListener('loadedmetadata', handleLoadedMetadata);
      audio.removeEventListener('play', handlePlay);
      audio.removeEventListener('pause', handlePause);
      audio.removeEventListener('ended', handleEnded);
      audio.removeEventListener('error', handleError);
      audio.pause();
      audio.removeAttribute('src');
      audio.load();
      // Отключаем усилитель от вывода; source-нода остаётся привязанной к
      // элементу и переиспользуется, если этот же элемент смонтируется снова.
      graph.gain.disconnect();

      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [record.id]);

  // Применяем сохранённую громкость к усилителю при каждом её изменении.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) {
      return;
    }

    const graph = audioGraphs.get(audio);
    if (graph) {
      graph.gain.gain.value = clamp(volume, 0, MAX_VOLUME);
    }
  }, [volume]);

  const togglePlay = () => {
    const audio = audioRef.current;
    if (!audio || hasError) {
      return;
    }

    if (audio.paused) {
      void getAudioContext().resume();
      void audio.play().catch(() => {});
    } else {
      audio.pause();
    }
  };

  const seek = (time: number) => {
    const audio = audioRef.current;
    if (!audio || hasError) {
      return;
    }

    isSeekingRef.current = true;
    const nextTime = clamp(time, 0, duration);
    audio.currentTime = nextTime;
    setCurrentTime(nextTime);
  };

  const finishSeek = () => {
    isSeekingRef.current = false;
  };

  const changeVolume = (nextVolume: number) => {
    setVolume(clamp(nextVolume, 0, MAX_VOLUME));
  };

  return {
    audioRef,
    duration,
    currentTime,
    isPlaying,
    hasError,
    volume,
    togglePlay,
    seek,
    finishSeek,
    changeVolume,
  };
};
