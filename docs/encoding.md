# Encoding

All project text files are UTF-8.

## Rules

- Keep `.editorconfig` as the source of truth for file encoding.
- VS Code must use `files.encoding: utf8` and must not guess encodings automatically.
- Before reading Cyrillic text with Windows PowerShell, use `scripts/powershell-utf8.cmd`.
- Do not trust PowerShell `Get-Content` output for Cyrillic text when the UTF-8 wrapper was not used.
- For command shell preferences, follow [command-execution.md](command-execution.md).
- Before replacing Cyrillic text, verify the actual file content as UTF-8 or run `npm.cmd run encoding:check`.
- Do not rewrite a file just because terminal output looks like mojibake.

## Checks

Run:

```bash
npm.cmd run encoding:check
```

The check scans text files for common UTF-8/Windows-codepage mojibake sequences.
It is also included in `npm.cmd run check`.
