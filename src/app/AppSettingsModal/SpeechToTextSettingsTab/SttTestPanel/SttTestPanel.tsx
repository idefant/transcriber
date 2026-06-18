import { type FC, useRef, useState } from 'react';
import { Alert, Button, Space, Typography, Upload, type UploadFile } from 'antd';
import { MicIcon, RotateCcwIcon, SquareIcon, UploadIcon } from 'lucide-react';

import { useProcessing } from '#/app/processingContext';
import * as processingApi from '#/shared/processingApi';

import styles from './SttTestPanel.module.scss';

interface LastAudio {
  audio: Uint8Array;
  fileName: string;
}

const formatElapsed = (elapsedMs: number) =>
  elapsedMs < 1000 ? `${elapsedMs} мс` : `${(elapsedMs / 1000).toFixed(1)} с`;

const SttTestPanel: FC = () => {
  const { config } = useProcessing();
  const [isRecording, setIsRecording] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [result, setResult] = useState<string>();
  const [error, setError] = useState<string>();
  const [elapsedMs, setElapsedMs] = useState(0);
  const [lastAudio, setLastAudio] = useState<LastAudio>();
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  const canRun = Boolean(config.stt.providerId && config.stt.modelKey);
  const canRepeat = canRun && !isRunning && !isRecording && lastAudio !== undefined;

  const runTest = async (audio: Uint8Array, fileName: string) => {
    if (!config.stt.providerId || !config.stt.modelKey) return;

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
            setError('Не удалось прочитать аудиозапись');
          });
      };

      mediaRecorderRef.current = recorder;
      recorder.start();
      setIsRecording(true);
      setResult(undefined);
      setError(undefined);
    } catch {
      setError('Не удалось получить доступ к микрофону');
    }
  };

  const handleStopRecording = () => {
    mediaRecorderRef.current?.stop();
    setIsRecording(false);
  };

  const handleUpload = (file: UploadFile) => {
    const rawFile = file.originFileObj ?? (file as unknown as File);

    rawFile
      .arrayBuffer()
      .then((buffer) => runTest(new Uint8Array(buffer), rawFile.name))
      .catch(() => {
        setError('Не удалось прочитать файл');
      });

    return false; // prevent default antd upload
  };

  const handleRepeat = () => {
    if (!lastAudio) return;

    void runTest(lastAudio.audio, lastAudio.fileName);
  };

  return (
    <div className={styles.panel}>
      <Typography.Text strong>Тест конфигурации</Typography.Text>

      <Space wrap>
        {isRecording ? (
          <Button
            danger
            icon={<SquareIcon size={16} strokeWidth={2} />}
            onClick={handleStopRecording}
          >
            Остановить запись
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
            Записать голос
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
            Загрузить файл
          </Button>
        </Upload>

        <Button
          disabled={!canRepeat}
          icon={<RotateCcwIcon size={16} strokeWidth={2} />}
          onClick={handleRepeat}
        >
          Повторить
        </Button>
      </Space>

      {!canRun && <Alert showIcon title="Выберите провайдера и модель выше" type="warning" />}

      {error !== undefined && <Alert showIcon title={error} type="error" />}

      {result !== undefined && (
        <div className={styles.result}>
          <Typography.Text type="secondary">
            Результат: ({formatElapsed(elapsedMs)})
          </Typography.Text>
          <Typography.Paragraph className={styles.resultText}>{result}</Typography.Paragraph>
        </div>
      )}
    </div>
  );
};

export default SttTestPanel;
