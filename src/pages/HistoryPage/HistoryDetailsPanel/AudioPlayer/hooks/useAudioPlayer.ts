import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { clamp } from 'lodash-es';

import { getHistoryAudioData } from '#/shared/historyApi';

import type { HistoryRecord } from '#/models/History';
import { usePlayerStore } from '#/stores';

/** Максимальная громкость плеера: 1.5 соответствует усилению до 150%. */
export const MAX_VOLUME = 1.5;

// Барьер общий для поколений hook: новая запись ждёт закрытия старого pipeline.
// Ошибка close оставляет барьер отклонённым, запрещая накопление живых контекстов.
let contextRelease: Promise<void> = Promise.resolve();
let playerGeneration = 0;

type PlayerEvent =
  | 'control'
  | 'load'
  | 'start'
  | 'stop'
  | 'end'
  | 'contextCreated'
  | 'contextState'
  | 'contextClosed'
  | 'failure'
  | 'hide'
  | 'cleanup';
type PlayerReason = 'play' | 'pause' | 'seek' | 'hidden' | 'ended' | 'cleanup' | 'load' | 'context';

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
  /** Идёт ли воспроизведение прямо сейчас, включая ожидающий запуск. */
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
 * Держит выходной pipeline только во время прослушивания. Пауза, завершение и
 * скрытие окна закрывают контекст; продолжение использует сохранённый буфер
 * и позицию. Токен запроса отменяет устаревшие загрузки, resume и перемотки.
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
    const generation = ++playerGeneration;
    const appWindow = getCurrentWindow();
    let cancelled = false;
    let wantsPlay = false;
    let pendingPlay = false;
    let seeking = false;
    let request = 0;
    let offset = 0;
    let startedAt = 0;
    let frame: number | undefined;
    let endTimer: ReturnType<typeof setTimeout> | undefined;
    let buffer: AudioBuffer | undefined;
    let context: AudioContext | undefined;
    let gain: GainNode | undefined;
    let source: AudioBufferSourceNode | undefined;
    let unlisten: (() => void) | undefined;
    let bytes: Promise<ArrayBuffer> | undefined;
    const isCurrent = (token: number) => !cancelled && token === request && wantsPlay;

    const log = (event: PlayerEvent, reason: PlayerReason, playbackContext = context) => {
      const state = playbackContext?.state;
      void invoke('log_player_event', {
        metadata: {
          event,
          reason,
          generation,
          request,
          positionMs: Math.round(offset * 1000),
          durationMs: Math.round((buffer?.duration ?? fallbackDuration) * 1000),
          contextState: ['running', 'suspended', 'interrupted', 'closed'].includes(state ?? '')
            ? state
            : null,
          sampleRate: playbackContext?.sampleRate ?? null,
          baseLatencyMs: playbackContext ? playbackContext.baseLatency * 1000 : null,
        },
      }).catch(() => {
        // Диагностика никогда не должна менять воспроизведение.
      });
    };

    const position = () =>
      source && context && buffer
        ? clamp(offset + context.currentTime - startedAt, 0, buffer.duration)
        : offset;

    const releaseContext = (reason: PlayerReason) => {
      const oldContext = context;
      const oldGain = gain;
      context = undefined;
      gain = undefined;
      if (gainRef.current === oldGain) gainRef.current = null;
      oldGain?.disconnect();
      if (!oldContext) return;
      // close вызывается немедленно, даже если resume ещё не разрешился.
      // Барьер не включает resume: иначе зависшее возобновление удержит pipeline.
      const closing = oldContext.close();
      contextRelease = Promise.all([contextRelease, closing]).then(() => {
        log('contextClosed', reason, oldContext);
        return;
      });
      void contextRelease.catch(() => {
        log('failure', 'context', oldContext);
      });
    };

    const stop = (reason: PlayerReason) => {
      ++request;
      if (endTimer !== undefined) clearTimeout(endTimer);
      endTimer = undefined;
      pendingPlay = false;
      offset = position();
      if (frame !== undefined) cancelAnimationFrame(frame);
      frame = undefined;
      if (gain && context) gain.gain.setValueAtTime(0, context.currentTime);
      const oldSource = source;
      source = undefined;
      if (oldSource) {
        oldSource.stop();
        oldSource.disconnect();
      }
      log('stop', reason);
      releaseContext(reason);
    };

    const pause = (reason: PlayerReason) => {
      wantsPlay = false;
      stop(reason);
      if (!cancelled) {
        setCurrentTime(offset);
        setIsPlaying(false);
      }
    };

    const hidden = () => {
      log('hide', 'hidden');
      pause('hidden');
    };
    const visibilityChanged = () => {
      if (document.hidden) hidden();
    };

    const updatePosition = () => {
      if (cancelled || !source) return;
      if (!seeking) setCurrentTime(position());
      frame = requestAnimationFrame(updatePosition);
    };

    const play = async () => {
      const token = ++request;
      pendingPlay = true;
      try {
        // Подписка готова до первой проверки видимости и до любой загрузки.
        await listening;
        if (!isCurrent(token)) return;
        if (token === 1) {
          setCurrentTime(0);
          setDuration(fallbackDuration);
          setHasError(false);
        }
        setIsPlaying(true);
        if (!(await appWindow.isVisible())) {
          if (isCurrent(token)) hidden();
          return;
        }
        if (!isCurrent(token)) return;
        let audioBytes: ArrayBuffer | undefined;
        if (!buffer) {
          bytes ??= getHistoryAudioData(record.id);
          audioBytes = await bytes;
          if (!isCurrent(token)) return;
        }
        await contextRelease;
        if (!isCurrent(token)) return;
        const playbackContext = new AudioContext({ latencyHint: 'interactive' });
        context = playbackContext;
        log('contextCreated', 'play');
        playbackContext.addEventListener('statechange', () => {
          log('contextState', 'context', playbackContext);
        });
        if (!buffer) {
          log('load', 'load');
          // decodeAudioData забирает ArrayBuffer; для отменённой загрузки сохраняем оригинал.
          if (!audioBytes) throw new Error('Audio data unavailable');
          // ArrayBuffer не итерируемый: slice копирует байты без передачи оригинала декодеру.
          // eslint-disable-next-line unicorn/prefer-spread
          const decoded = await playbackContext.decodeAudioData(audioBytes.slice(0));
          if (!isCurrent(token)) return;
          if (!Number.isFinite(decoded.duration) || decoded.duration <= 0) {
            throw new Error('Invalid audio duration');
          }
          buffer = decoded;
          offset = clamp(offset, 0, decoded.duration);
          setDuration(decoded.duration);
          setCurrentTime(offset);
        }
        const playbackBuffer = buffer;
        await playbackContext.resume();
        if (!isCurrent(token)) return;
        const visible = await appWindow.isVisible();
        if (!isCurrent(token)) return;
        if (!visible) {
          hidden();
          return;
        }
        if (playbackContext.state !== 'running') throw new Error('Audio context is not running');
        const playbackGain = playbackContext.createGain();
        playbackGain.gain.value = clamp(usePlayerStore.getState().volume, 0, MAX_VOLUME);
        playbackGain.connect(playbackContext.destination);
        gain = playbackGain;
        gainRef.current = playbackGain;
        if (offset >= playbackBuffer.duration) offset = 0;
        const nextSource = playbackContext.createBufferSource();
        nextSource.buffer = playbackBuffer;
        nextSource.connect(playbackGain);
        nextSource.addEventListener(
          'ended',
          () => {
            if (cancelled || source !== nextSource) return;
            nextSource.disconnect();
            source = undefined;
            wantsPlay = false;
            offset = playbackBuffer.duration;
            if (frame !== undefined) cancelAnimationFrame(frame);
            frame = undefined;
            setCurrentTime(offset);
            setIsPlaying(false);
            log('end', 'ended');
            // ended приходит после рендеринга, до выхода последних сэмплов.
            // Только естественному концу даём ограниченное время на drain.
            const outputLatency = playbackContext.outputLatency || 0;
            const drainMs = clamp(
              (playbackContext.baseLatency + outputLatency) * 1000 + 250,
              250,
              2000,
            );
            endTimer = setTimeout(() => {
              endTimer = undefined;
              releaseContext('ended');
            }, drainMs);
          },
          { once: true },
        );
        startedAt = playbackContext.currentTime;
        nextSource.start(0, offset);
        source = nextSource;
        pendingPlay = false;
        log('start', 'play');
        if (!seeking) setCurrentTime(offset);
        updatePosition();
      } catch {
        if (!isCurrent(token)) return;
        log('failure', buffer ? 'context' : 'load');
        pause(buffer ? 'context' : 'load');
        if (!buffer) setHasError(true);
      }
    };

    const controls: PlaybackControls = {
      togglePlay: () => {
        log('control', source || pendingPlay ? 'pause' : 'play');
        if (source || pendingPlay) {
          pause('pause');
        } else {
          // Повторный запуск до конца drain сначала закрывает прежний pipeline.
          stop('play');
          wantsPlay = true;
          void play();
        }
      },
      seek: (time) => {
        log('control', 'seek');
        seeking = true;
        stop('seek');
        setIsPlaying(false);
        offset = clamp(time, 0, buffer?.duration ?? fallbackDuration);
        setCurrentTime(offset);
        if (buffer && offset >= buffer.duration) wantsPlay = false;
        if (wantsPlay) void play();
      },
      finishSeek: () => {
        seeking = false;
        setCurrentTime(position());
      },
    };
    controlsRef.current = controls;
    document.addEventListener('visibilitychange', visibilityChanged);
    const listening = appWindow.listen('main-window-hidden', hidden).then((remove) => {
      if (cancelled) remove();
      else unlisten = remove;
      return;
    });
    wantsPlay = true;
    void play();

    return () => {
      log('cleanup', 'cleanup');
      cancelled = true;
      wantsPlay = false;
      stop('cleanup');
      unlisten?.();
      document.removeEventListener('visibilitychange', visibilityChanged);
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
