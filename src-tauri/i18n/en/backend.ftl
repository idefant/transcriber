tray-open = Open application
tray-copy-latest = Copy latest transcription
tray-exit = Exit

notification-config-error-title = Couldn't start dictation
notification-config-error-body = Check the { $section } settings: { $message }. Click to open settings.
config-section-speech-to-text = speech-to-text
config-section-post-processing = post-processing
config-section-dictionary = dictionary

config-error-custom-provider-url-required = URL is required for custom provider
config-error-model-not-available-for-provider = Model is not available for this provider
config-error-provider-is-not-openrouter = Provider is not an OpenRouter connection
config-error-model-not-found-in-catalog = Model not found in catalog
config-error-model-not-selected = Model is not selected
config-error-provider-api-key-not-found = Provider API key was not found
config-error-provider-not-found = Provider was not found
config-error-provider-not-selected = Provider is not selected
config-error-provider-url-not-found = Provider URL was not found
config-error-selected-model-is-not-post-processing = Selected model is not a post-processing model
config-error-selected-model-is-not-speech-to-text = Selected model is not a speech-to-text model

provider-validation-success = Configuration is valid. Models found: { $count }.
validation-api-key-required = API key is required
validation-field-required = { $field } is required
provider-response-unsupported-models = Provider returned an unsupported models response.
provider-request-failed = Provider request failed with status { $status }.
provider-request-failed-with-body = Provider request failed with status { $status }: { $body }
header-format-invalid = Header must use `Name: value` format: { $line }
header-name-invalid = Invalid header name `{ $name }`: { $error }
header-value-invalid = Invalid header value for `{ $name }`: { $error }

mime-type-invalid = Invalid MIME type: { $error }
stt-request-failed = STT request failed with status { $status }
stt-prompt-token-limit-exceeded = The speech-to-text prompt is too large
post-process-request-failed = Post-process request failed with status { $status }

history-record-not-found = History record was not found
history-audio-file-not-found = Audio file was not found
history-open-file-explorer-failed = Could not open File Explorer: { $error }
history-open-audio-location-failed = Could not open audio location: { $error }
history-transcription-required-before-post-processing = Transcription result is required before post-processing
history-post-processing-disabled = Post-processing is disabled

recording-no-default-input-device = No default input device is available
recording-input-device-config-read-failed = Could not read input device config: { $error }
recording-unsupported-input-sample-format = Unsupported input sample format: { $format }
recording-start-failed = Could not start recording: { $error }
recording-read-samples-failed = Could not read recorded audio samples
recording-no-audio-captured = Recording did not capture any audio
recording-no-speech-detected = Recording did not contain speech
recording-speech-too-short = Speech in the recording is too short
recording-vad-failed = Could not analyze speech in the recording
recording-build-input-stream-failed = Could not build input stream: { $error }
recording-create-wav-writer-failed = Could not create WAV writer: { $error }
recording-write-wav-sample-failed = Could not write WAV sample: { $error }
recording-finalize-wav-failed = Could not finalize WAV recording: { $error }

shortcut-windows-only = Suppressing dictation shortcuts is only implemented on Windows in this version
shortcut-single-main-key-required = Shortcut can contain only one main key
shortcut-non-modifier-key-required = Shortcut must contain a non-modifier key
shortcut-key-unsupported = Unsupported shortcut key: { $value }
shortcut-hook-state-lock-failed = Could not lock shortcut hook state
shortcut-cancel-hotkey-state-lock-failed = Could not lock cancel hotkey state
shortcut-paste-latest-hotkey-state-lock-failed = Could not lock paste latest hotkey state
shortcut-copy-latest-hotkey-state-lock-failed = Could not lock copy latest hotkey state
shortcut-repeat-latest-hotkey-state-lock-failed = Could not lock repeat latest hotkey state
shortcut-hook-runtime-lock-failed = Could not lock shortcut hook runtime
shortcut-install-windows-keyboard-hook-failed = Could not install Windows keyboard hook

clipboard-paste-windows-only = Dictation paste is only implemented on Windows in this version
clipboard-copy-windows-only = Clipboard copy is only implemented on Windows in this version
clipboard-open-failed = Could not open Windows clipboard
clipboard-set-data-failed = Could not set Windows clipboard data
clipboard-send-ctrl-v-failed = Could not send Ctrl+V input

dictation-state-lock-failed = Could not lock dictation state
dictation-active-hold-shortcut-state-lock-failed = Could not lock active hold shortcut state
dictation-stt-provider-and-model-not-selected = Speech-to-text provider and model are not selected

overlay-no-monitor-available = No monitor is available for recording overlay

tray-state-lock-failed = Could not lock tray state
tray-icon-not-found = Tray icon was not found
main-window-not-found = Main window was not found

autostart-open-registry-key-failed = Could not open Windows startup registry key: { $status }
autostart-set-registry-value-failed = Could not set Windows startup registry value: { $status }
autostart-delete-registry-value-failed = Could not delete Windows startup registry value: { $status }

updater-no-pending-update = No pending update found. Call check_for_update first.
prompt-defaults-invalid = Invalid prompt defaults: { $error }
