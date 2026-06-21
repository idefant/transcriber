# Debug Logging

Debug logs are intended for local troubleshooting of Speech-to-Text and post-processing model calls.

## Format

Logs use append-only `.log` files instead of a single valid JSON document.

Each event is written as:

```text
================================================================================
2026-06-21T18:42:12.123Z speechToText.response operationId=... historyRecordId=...
--------------------------------------------------------------------------------
{
  "timestamp": "...",
  "event": "speechToText.response"
}
```

The file as a whole is not valid JSON. Each event payload is pretty-printed JSON, which keeps long request and response objects readable without requiring the application to reread and rewrite a growing file for every event.

## Session Files

The current logging session writes to one debug log file. A new file is created lazily on the first logged event after logging is enabled.

Switching logging off closes the current logging session. Switching it on again during the same process creates a new file when logging first writes again. Restarting the application also starts a new logging session.

## Secrets

Debug logs must not include API keys, `Authorization`, audio bytes, or custom header values.

Custom request headers may be logged by name with a boolean that indicates whether a value was present. This keeps provider configuration debuggable without storing secret values.

## History Correlation

Dictation creates the history record ID before model calls start. Model events include this ID and the original recording timestamp, so a log entry can be matched to the final history record even when an error happens before the record is saved.
