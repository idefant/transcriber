import { type FC, useState } from 'react';
import { Button, Popover, Slider, Tooltip } from 'antd';
import clsx from 'clsx';
import { PauseIcon, PlayIcon, Volume2Icon, VolumeXIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { MAX_VOLUME, useAudioPlayer } from './hooks/useAudioPlayer';

import styles from './AudioPlayer.module.scss';

import type { HistoryRecord } from '#/models/History';

interface AudioPlayerProps {
  record: HistoryRecord;
}

// Форматирует длительность в секундах в строку вида `mm:ss`.
const formatTime = (seconds: number): string => {
  const safeSeconds = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  const minutes = Math.floor(safeSeconds / 60);
  const rest = safeSeconds % 60;
  return `${String(minutes).padStart(2, '0')}:${String(rest).padStart(2, '0')}`;
};

const AudioPlayer: FC<AudioPlayerProps> = ({ record }) => {
  const { t } = useTranslation();
  const {
    audioRef,
    changeVolume,
    currentTime,
    duration,
    finishSeek,
    hasError,
    isPlaying,
    seek,
    togglePlay,
    volume,
  } = useAudioPlayer(record);

  // Поповер громкости открыт, если на кнопку наведена мышь ИЛИ пользователь тянет
  // ползунок. Второе условие удерживает поповер открытым, даже когда курсор
  // выходит за его пределы во время перетаскивания: иначе antd размонтировал бы
  // слайдер прямо посреди drag, что рвёт регулировку и роняет интерфейс.
  const [isVolumeHovered, setIsVolumeHovered] = useState(false);
  const [isVolumeDragging, setIsVolumeDragging] = useState(false);
  const isVolumeOpen = isVolumeHovered || isVolumeDragging;

  const volumeControl = (
    <div className={styles.volumePopover}>
      <Slider
        className={clsx(volume > 1 && styles.boosted)}
        marks={{ 0.5: ' ', 1: ' ' }}
        max={MAX_VOLUME}
        min={0}
        step={0.01}
        tooltip={{ formatter: (value) => `${Math.round((value ?? 0) * 100)}%` }}
        value={volume}
        vertical
        onChange={(value) => {
          setIsVolumeDragging(true);
          changeVolume(value);
        }}
        onChangeComplete={() => {
          setIsVolumeDragging(false);
        }}
      />
    </div>
  );

  return (
    <div className={styles.player}>
      {/* Скрытый элемент воспроизведения. Субтитры к пользовательским аудиозаписям не предусмотрены. */}
      {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
      <audio ref={audioRef} />

      {hasError ? (
        <span className={styles.unavailable}>{t('history.details.audioUnavailable')}</span>
      ) : (
        <>
          <Tooltip title={isPlaying ? t('history.details.pause') : t('history.details.play')}>
            <Button
              aria-label={isPlaying ? t('history.details.pause') : t('history.details.play')}
              icon={
                isPlaying ? (
                  <PauseIcon size={16} strokeWidth={2} />
                ) : (
                  <PlayIcon size={16} strokeWidth={2} />
                )
              }
              size="small"
              type="text"
              onClick={togglePlay}
            />
          </Tooltip>

          <span className={styles.time}>{formatTime(currentTime)}</span>

          <Slider
            className={styles.progress}
            max={duration}
            min={0}
            step={0.1}
            tooltip={{ formatter: (value) => formatTime(value ?? 0) }}
            value={currentTime}
            onChange={seek}
            onChangeComplete={finishSeek}
          />

          <span className={styles.time}>{formatTime(duration)}</span>

          <Popover
            content={volumeControl}
            open={isVolumeOpen}
            placement="top"
            trigger="hover"
            onOpenChange={setIsVolumeHovered}
          >
            <Button
              aria-label={t('history.details.volume')}
              icon={
                volume === 0 ? (
                  <VolumeXIcon size={16} strokeWidth={2} />
                ) : (
                  <Volume2Icon size={16} strokeWidth={2} />
                )
              }
              size="small"
              type="text"
            />
          </Popover>
        </>
      )}
    </div>
  );
};

export default AudioPlayer;
