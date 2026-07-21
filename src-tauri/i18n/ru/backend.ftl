tray-open = Открыть приложение
tray-copy-latest = Скопировать последнюю расшифровку
tray-exit = Выход

notification-config-error-title = Не удалось начать распознавание
notification-config-error-body = Проверьте настройки { $section }: { $message }. Нажмите, чтобы открыть настройки.
config-section-speech-to-text = распознавания речи
config-section-post-processing = постобработки
config-section-dictionary = словаря

config-error-custom-provider-url-required = Для Custom-провайдера не задан URL
config-error-model-not-available-for-provider = Модель недоступна для этого провайдера
config-error-provider-is-not-openrouter = Провайдер не является подключением OpenRouter
config-error-model-not-found-in-catalog = Модель не найдена в каталоге
config-error-model-not-selected = Модель не выбрана
config-error-provider-api-key-not-found = API-ключ провайдера не найден
config-error-provider-not-found = Провайдер не найден
config-error-provider-not-selected = Провайдер не выбран
config-error-provider-url-not-found = URL провайдера не найден
config-error-selected-model-is-not-post-processing = Выбранная модель не относится к постобработке
config-error-selected-model-is-not-speech-to-text = Выбранная модель не относится к распознаванию речи

provider-validation-success = Конфигурация корректна. Найдено моделей: { $count }.
validation-api-key-required = Требуется API-ключ
validation-field-required = Поле «{ $field }» обязательно
provider-response-unsupported-models = Провайдер вернул неподдерживаемый ответ со списком моделей.
provider-request-failed = Запрос к провайдеру завершился ошибкой со статусом { $status }.
provider-request-failed-with-body = Запрос к провайдеру завершился ошибкой со статусом { $status }: { $body }
header-format-invalid = Заголовок должен быть в формате `Name: value`: { $line }
header-name-invalid = Некорректное имя заголовка `{ $name }`: { $error }
header-value-invalid = Некорректное значение заголовка `{ $name }`: { $error }

mime-type-invalid = Некорректный MIME-тип: { $error }
stt-request-failed = STT-запрос завершился ошибкой со статусом { $status }
stt-prompt-token-limit-exceeded = Превышен размер промпта распознавания речи
post-process-request-failed = Запрос постобработки завершился ошибкой со статусом { $status }

history-record-not-found = Запись истории не найдена
history-audio-file-not-found = Аудиофайл не найден
history-open-file-explorer-failed = Не удалось открыть Проводник: { $error }
history-open-audio-location-failed = Не удалось открыть папку с аудио: { $error }
history-transcription-required-before-post-processing = Перед постобработкой нужен результат распознавания
history-post-processing-disabled = Постобработка отключена

recording-no-default-input-device = Устройство ввода по умолчанию недоступно
recording-input-device-config-read-failed = Не удалось прочитать конфигурацию устройства ввода: { $error }
recording-unsupported-input-sample-format = Неподдерживаемый формат входных сэмплов: { $format }
recording-start-failed = Не удалось начать запись: { $error }
recording-read-samples-failed = Не удалось прочитать записанные аудиосэмплы
recording-no-audio-captured = Во время записи не было захвачено аудио
recording-vad-failed = Не удалось проанализировать речь в записи
recording-build-input-stream-failed = Не удалось создать входной аудиопоток: { $error }
recording-create-wav-writer-failed = Не удалось создать WAV writer: { $error }
recording-write-wav-sample-failed = Не удалось записать WAV-сэмпл: { $error }
recording-finalize-wav-failed = Не удалось завершить запись WAV: { $error }

shortcut-windows-only = Подавление горячих клавиш диктовки в этой версии реализовано только для Windows
shortcut-single-main-key-required = Горячая клавиша может содержать только одну основную клавишу
shortcut-non-modifier-key-required = Горячая клавиша должна содержать немодификатор
shortcut-key-unsupported = Неподдерживаемая клавиша горячей клавиши: { $value }
shortcut-hook-state-lock-failed = Не удалось заблокировать состояние hook горячих клавиш
shortcut-cancel-hotkey-state-lock-failed = Не удалось заблокировать состояние горячей клавиши отмены
shortcut-paste-latest-hotkey-state-lock-failed = Не удалось заблокировать состояние горячей клавиши вставки последней записи
shortcut-copy-latest-hotkey-state-lock-failed = Не удалось заблокировать состояние горячей клавиши копирования последней записи
shortcut-repeat-latest-hotkey-state-lock-failed = Не удалось заблокировать состояние горячей клавиши повтора последней записи
shortcut-hook-runtime-lock-failed = Не удалось заблокировать runtime hook горячих клавиш
shortcut-install-windows-keyboard-hook-failed = Не удалось установить Windows keyboard hook

clipboard-paste-windows-only = Вставка текста диктовки в этой версии реализована только для Windows
clipboard-copy-windows-only = Копирование в буфер обмена в этой версии реализовано только для Windows
clipboard-open-failed = Не удалось открыть буфер обмена Windows
clipboard-set-data-failed = Не удалось записать данные в буфер обмена Windows
clipboard-send-ctrl-v-failed = Не удалось отправить сочетание Ctrl+V

dictation-state-lock-failed = Не удалось заблокировать состояние диктовки
dictation-active-hold-shortcut-state-lock-failed = Не удалось заблокировать состояние активной удерживаемой горячей клавиши
dictation-stt-provider-and-model-not-selected = Провайдер и модель распознавания речи не выбраны

overlay-no-monitor-available = Нет доступного монитора для оверлея записи

tray-state-lock-failed = Не удалось заблокировать состояние трей-иконки
tray-icon-not-found = Иконка трея не найдена
main-window-not-found = Главное окно не найдено

autostart-open-registry-key-failed = Не удалось открыть ключ автозапуска Windows в реестре: { $status }
autostart-set-registry-value-failed = Не удалось записать значение автозапуска Windows в реестр: { $status }
autostart-delete-registry-value-failed = Не удалось удалить значение автозапуска Windows из реестра: { $status }

updater-no-pending-update = Нет ожидающего обновления. Сначала вызовите check_for_update.
prompt-defaults-invalid = Некорректные prompt defaults: { $error }
