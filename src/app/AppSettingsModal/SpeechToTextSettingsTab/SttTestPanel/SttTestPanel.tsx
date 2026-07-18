import { type FC, useEffect, useRef, useState } from 'react';
import { Alert, Button, Space, Typography, Upload, type UploadFile } from 'antd';
import { MicIcon, RotateCcwIcon, SquareIcon, UploadIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import * as processingApi from '#/shared/processingApi';

import styles from './SttTestPanel.module.scss';

import { useDictionaryStore, useProcessing } from '#/stores';

interface LastAudio {
  audio: Uint8Array;
  fileName: string;
}

const SttTestPanel: FC = () => {
  const { config } = useProcessing();
  const dictionaryWords = useDictionaryStore((state) => state.words);
  const { t } = useTranslation();
  const [isRecording, setIsRecording] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [result, setResult] = useState<string>();
  const [error, setError] = useState<string>();
  const [elapsedMs, setElapsedMs] = useState(0);
  const [lastAudio, setLastAudio] = useState<LastAudio>();
  const [promptAnalysis, setPromptAnalysis] = useState<processingApi.SttPromptAnalysis | null>(
    null,
  );
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  const isPromptLimitExceeded = (promptAnalysis?.excludedTokenCount ?? 0) > 0;
  const canRun = Boolean(config.stt.providerId && config.stt.modelKey) && !isPromptLimitExceeded;
  const canRepeat = canRun && !isRunning && !isRecording && lastAudio !== undefined;
  const formatElapsed = (value: number) =>
    value < 1000
      ? t('common.milliseconds', { value })
      : t('common.seconds', { value: (value / 1000).toFixed(1) });

  useEffect(() => {
    let cancelled = false;

    void processingApi
      .analyzeSttPrompt()
      .then((analysis) => {
        if (!cancelled) setPromptAnalysis(analysis);
        return;
      })
      .catch(() => {
        if (!cancelled) setPromptAnalysis(null);
      });

    return () => {
      cancelled = true;
    };
  }, [config.stt.modelKey, config.stt.systemPrompt, dictionaryWords]);

  const runTest = async (audio: Uint8Array, fileName: string) => {
    if (!config.stt.providerId || !config.stt.modelKey || isPromptLimitExceeded) return;

    setLastAudio({ audio, fileName });
    setIsRunning(true);
    setResult(undefined);
    setError(undefined);

    const startedAt = performance.now();

    try {
      const text = await processingApi.runSttTest({ audio: [...audio], fileName });

      setElapsedMs(Math.round(performance.now() - startedAt));
      setResult(text);
    } catch (unknownError) {
      setError(unknownError instanceof Error ? unknownError.message : String(unknownError));
    } finally {
      setIsRunning(false);
    }
  };

  const handleStartRecording = async () => {
    if (isPromptLimitExceeded) return;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const recorder = new MediaRecorder(stream);

      chunksRef.current = [];
      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunksRef.current.push(e.data);
      };
      recorder.onstop = () => {
        for (const track of stream.getTracks()) {
          track.stop();
        }

        const blob = new Blob(chunksRef.current, { type: 'audio/webm' });

        blob
          .arrayBuffer()
          .then((buffer) => runTest(new Uint8Array(buffer), 'recording.webm'))
          .catch(() => {
            setError(t('settings.tests.readRecordingError'));
          });
      };

      mediaRecorderRef.current = recorder;
      recorder.start();
      setIsRecording(true);
      setResult(undefined);
      setError(undefined);
    } catch {
      setError(t('settings.tests.microphoneError'));
    }
  };

  const handleStopRecording = () => {
    mediaRecorderRef.current?.stop();
    setIsRecording(false);
  };

  const handleUpload = (file: UploadFile) => {
    if (isPromptLimitExceeded) return false;
    const rawFile = file.originFileObj ?? (file as unknown as File);

    rawFile
      .arrayBuffer()
      .then((buffer) => runTest(new Uint8Array(buffer), rawFile.name))
      .catch(() => {
        setError(t('settings.tests.readFileError'));
      });

    return false; // предотвращаем стандартную загрузку antd
  };

  const handleRepeat = () => {
    if (!lastAudio || isPromptLimitExceeded) return;

    void runTest(lastAudio.audio, lastAudio.fileName);
  };

  return (
    <div className={styles.panel}>
      <Typography.Text strong>{t('settings.tests.title')}</Typography.Text>

      <Space wrap>
        {isRecording ? (
          <Button
            danger
            icon={<SquareIcon size={16} strokeWidth={2} />}
            onClick={handleStopRecording}
          >
            {t('settings.tests.stopRecording')}
          </Button>
        ) : (
          <Button
            disabled={!canRun || isRunning}
            icon={<MicIcon size={16} strokeWidth={2} />}
            loading={isRunning}
            onClick={() => {
              void handleStartRecording();
            }}
          >
            {t('settings.tests.recordVoice')}
          </Button>
        )}

        <Upload
          accept="audio/*,.mp3,.wav,.ogg,.flac,.webm,.m4a"
          maxCount={1}
          showUploadList={false}
          beforeUpload={(file) => {
            void handleUpload(file as UploadFile);
            return false;
          }}
        >
          <Button
            disabled={!canRun || isRunning || isRecording}
            icon={<UploadIcon size={16} strokeWidth={2} />}
          >
            {t('settings.tests.uploadFile')}
          </Button>
        </Upload>

        <Button
          disabled={!canRepeat}
          icon={<RotateCcwIcon size={16} strokeWidth={2} />}
          onClick={handleRepeat}
        >
          {t('settings.tests.repeat')}
        </Button>
      </Space>

      {!canRun && !isPromptLimitExceeded && (
        <Alert showIcon title={t('settings.processing.selectProviderAndModel')} type="warning" />
      )}

      {error !== undefined && <Alert showIcon title={error} type="error" />}

      {result !== undefined && (
        <div className={styles.result}>
          <Typography.Text type="secondary">
            {t('settings.tests.result', { elapsed: formatElapsed(elapsedMs) })}
          </Typography.Text>
          <Typography.Paragraph className={styles.resultText}>{result}</Typography.Paragraph>
        </div>
      )}
    </div>
  );
};

export default SttTestPanel;
