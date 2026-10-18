import { useEffect, useRef, useState } from 'react';
import { clamp } from 'lodash-es';

import { getHistoryAudioData } from '#/shared/historyApi';

import type { HistoryRecord } from '#/models/History';
import { usePlayerStore } from '#/stores';

/** Максимальная громкость плеера: 1.5 соответствует усилению до 150%. */
export const MAX_VOLUME = 1.5;

// Переиспользуем контекст между записями, чтобы не исчерпать лимит браузера.
let sharedAudioContext: AudioContext | undefined;

const getAudioContext = (): AudioContext => {
  sharedAudioContext ??= new AudioContext({ latencyHint: 'interactive' });
  return sharedAudioContext;
};

interface PlaybackControls {
  togglePlay: () => void;
  seek: (time: number) => void;
  finishSeek: () => void;
}

interface UseAudioPlayerResult {
  /** Длительность декодированного аудио в секундах, до загрузки — из данных записи. */
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
  /** Перематывает воспроизведение в секундах, в том числе во время перетаскивания. */
  seek: (time: number) => void;
  /** Возвращает синхронизацию позиции после перетаскивания ползунка. */
  finishSeek: () => void;
  /** Меняет громкость и запоминает её между записями и перезапусками. */
  changeVolume: (volume: number) => void;
}

/**
 * Декодирует сохранённую запись и воспроизводит её через Web Audio без очереди
 * HTMLAudioElement. Пауза останавливает источник, продолжение создаёт новый
 * с сохранённой позиции. Смена записи и размонтирование отключают аудиограф
 * и инвалидируют незавершённые загрузки и запросы запуска.
 */
export const useAudioPlayer = (record: HistoryRecord): UseAudioPlayerResult => {
  const volume = usePlayerStore((state) => state.volume);
  const setVolume = usePlayerStore((state) => state.setVolume);
  const controlsRef = useRef<PlaybackControls | null>(null);
  const gainRef = useRef<GainNode | null>(null);
  const fallbackDuration = record.audio.durationMs / 1000;
  const [duration, setDuration] = useState(fallbackDuration);
  const [currentTime, setCurrentTime] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let wantsPlay = true;
    let pendingPlay = false;
    let seeking = false;
    let request = 0;
    let offset = 0;
    let startedAt = 0;
    let frame: number | undefined;
    let buffer: AudioBuffer | undefined;
    let context: AudioContext | undefined;
    let gain: GainNode | undefined;
    let source: AudioBufferSourceNode | undefined;
    const isActive = () => !cancelled;

    const position = () =>
      source && context && buffer
        ? clamp(offset + context.currentTime - startedAt, 0, buffer.duration)
        : offset;

    const stop = () => {
      pendingPlay = false;
      offset = position();
      if (frame !== undefined) cancelAnimationFrame(frame);
      frame = undefined;
      if (source) {
        source.stop();
        source.disconnect();
        source = undefined;
      }
    };

    const updatePosition = () => {
      if (!isActive() || !source) return;
      if (!seeking) setCurrentTime(position());
      frame = requestAnimationFrame(updatePosition);
    };

    const play = async () => {
      const currentRequest = ++request;
      const playbackContext = context;
      const playbackGain = gain;
      const playbackBuffer = buffer;
      if (!playbackContext || !playbackGain || !playbackBuffer || source) return;
      pendingPlay = true;
      setIsPlaying(true);
      try {
        await playbackContext.resume();
      } catch {
        // После отказа autoplay пользователь может повторить запуск вручную.
        if (isActive() && currentRequest === request) {
          pendingPlay = false;
          wantsPlay = false;
          setIsPlaying(false);
        }
        return;
      }
      if (!isActive() || currentRequest !== request || !wantsPlay) return;
      pendingPlay = false;
      if (playbackContext.state !== 'running') {
        wantsPlay = false;
        setIsPlaying(false);
        return;
      }

      if (offset >= playbackBuffer.duration) offset = 0;
      const nextSource = playbackContext.createBufferSource();
      nextSource.buffer = playbackBuffer;
      nextSource.connect(playbackGain);
      nextSource.addEventListener(
        'ended',
        () => {
          if (!isActive() || source !== nextSource) return;
          nextSource.disconnect();
          source = undefined;
          wantsPlay = false;
          offset = playbackBuffer.duration;
          if (frame !== undefined) cancelAnimationFrame(frame);
          frame = undefined;
          setCurrentTime(offset);
          setIsPlaying(false);
        },
        { once: true },
      );
      try {
        startedAt = playbackContext.currentTime;
        nextSource.start(0, offset);
        source = nextSource;
        setIsPlaying(true);
        if (!seeking) setCurrentTime(offset);
        updatePosition();
      } catch {
        nextSource.disconnect();
        if (isActive()) {
          wantsPlay = false;
          setIsPlaying(false);
          setHasError(true);
        }
      }
    };

    const controls: PlaybackControls = {
      togglePlay: () => {
        if (source || pendingPlay) {
          wantsPlay = false;
          ++request;
          stop();
          setCurrentTime(offset);
          setIsPlaying(false);
        } else {
          wantsPlay = true;
          void play();
        }
      },
      seek: (time) => {
        seeking = true;
        ++request;
        stop();
        setIsPlaying(false);
        offset = clamp(time, 0, buffer?.duration ?? fallbackDuration);
        setCurrentTime(offset);
        if (buffer && offset >= buffer.duration) {
          wantsPlay = false;
          setIsPlaying(false);
        } else if (wantsPlay) {
          void play();
        }
      },
      finishSeek: () => {
        seeking = false;
        setCurrentTime(position());
      },
    };
    controlsRef.current = controls;

    const load = async () => {
      try {
        context = getAudioContext();
        gain = context.createGain();
        gain.gain.value = clamp(usePlayerStore.getState().volume, 0, MAX_VOLUME);
        gain.connect(context.destination);
        gainRef.current = gain;
        const bytes = await getHistoryAudioData(record.id);
        if (!isActive()) return;
        setCurrentTime(0);
        setDuration(fallbackDuration);
        setIsPlaying(false);
        setHasError(false);
        const decoded = await context.decodeAudioData(bytes);
        if (!isActive()) return;
        if (!Number.isFinite(decoded.duration) || decoded.duration <= 0) {
          throw new Error('Invalid audio duration');
        }
        buffer = decoded;
        offset = clamp(offset, 0, decoded.duration);
        setDuration(decoded.duration);
        setCurrentTime(offset);
        if (wantsPlay) await play();
      } catch {
        if (isActive()) {
          wantsPlay = false;
          stop();
          setIsPlaying(false);
          setHasError(true);
        }
      }
    };
    void load();

    return () => {
      cancelled = true;
      ++request;
      stop();
      gain?.disconnect();
      if (gainRef.current === gain) gainRef.current = null;
      if (controlsRef.current === controls) controlsRef.current = null;
    };
  }, [record.id, fallbackDuration]);

  useEffect(() => {
    if (gainRef.current) gainRef.current.gain.value = clamp(volume, 0, MAX_VOLUME);
  }, [volume]);

  return {
    duration,
    currentTime,
    isPlaying,
    hasError,
    volume,
    togglePlay: () => {
      if (!hasError) controlsRef.current?.togglePlay();
    },
    seek: (time) => {
      if (!hasError) controlsRef.current?.seek(time);
    },
    finishSeek: () => {
      controlsRef.current?.finishSeek();
    },
    changeVolume: (nextVolume) => {
      setVolume(clamp(nextVolume, 0, MAX_VOLUME));
    },
  };
};
