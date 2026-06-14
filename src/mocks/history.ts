import type { HistoryGroup } from '#/models/History';

export const historyGroups: HistoryGroup[] = [
  {
    date: '2026-06-13',
    label: '13 июня 2026',
    month: '2026-06',
    records: [
      {
        id: '13-1042',
        audio: {
          duration: '00:02:34',
          path: String.raw`C:\Users\inik3\Recordings\meeting-2026-06-13-1042.wav`,
        },
        postprocessing: {
          cost: '$0.0041',
          duration: '1.8 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenAI',
          text: 'Итоговая версия заметки: согласовать дизайн панели истории и подготовить моковые состояния.',
        },
        time: '10:42',
        transcription: {
          cost: '$0.0126',
          duration: '6.4 сек',
          isProcessing: true,
          model: 'whisper-large-v3',
          provider: 'Groq',
          text: 'Нужно согласовать внешний вид панели истории и подготовить несколько моковых состояний.',
        },
      },
      {
        id: '13-0915',
        audio: {
          duration: '00:00:58',
          path: String.raw`C:\Users\inik3\Recordings\quick-note-2026-06-13-0915.wav`,
        },
        postprocessing: {
          cost: '$0.0013',
          duration: '0.9 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenAI',
          text: 'Добавить проверку состояния кнопки повтора во время обработки.',
        },
        time: '09:15',
        transcription: {
          cost: '$0.0038',
          duration: '2.1 сек',
          isProcessing: false,
          model: 'whisper-1',
          provider: 'OpenAI',
          text: 'Добавить проверку состояния кнопки повтора во время обработки.',
        },
      },
    ],
  },
  {
    date: '2026-06-12',
    label: '12 июня 2026',
    month: '2026-06',
    records: [
      {
        id: '12-1810',
        audio: {
          duration: '00:04:17',
          path: String.raw`C:\Users\inik3\Recordings\planning-2026-06-12-1810.wav`,
        },
        postprocessing: {
          cost: '$0.0068',
          duration: '2.5 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenRouter',
          text: 'План: собрать страницу истории, правую панель деталей и моковые действия для элементов списка.',
        },
        time: '18:10',
        transcription: {
          cost: '$0.0184',
          duration: '8.7 сек',
          isProcessing: false,
          model: 'whisper-large-v3',
          provider: 'Groq',
          text: 'Собрать страницу истории, правую панель деталей и моковые действия для элементов списка.',
        },
      },
    ],
  },
  {
    date: '2026-05-30',
    label: '30 мая 2026',
    month: '2026-05',
    records: [
      {
        id: '30-1428',
        audio: {
          duration: '00:01:46',
          path: String.raw`C:\Users\inik3\Recordings\dictionary-2026-05-30-1428.wav`,
        },
        postprocessing: {
          cost: '$0.0029',
          duration: '1.2 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenAI',
          text: 'Словарь должен добавлять слова по Enter и через кнопку рядом с полем ввода.',
        },
        time: '14:28',
        transcription: {
          cost: '$0.0072',
          duration: '3.9 сек',
          isProcessing: false,
          model: 'whisper-1',
          provider: 'OpenAI',
          text: 'Словарь должен добавлять слова по Enter и через кнопку рядом с полем ввода.',
        },
      },
    ],
  },
];
