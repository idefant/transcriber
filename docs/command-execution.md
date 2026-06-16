# Command Execution

Prefer Git Bash for project commands when it is available:

```text
C:\Program Files\Git\bin\bash.exe
```

## Rules

- Use Bash for reading/searching files, inspecting text output, and running cross-platform commands.
- Use PowerShell when the command is Windows-specific, for example `npm.cmd`, Windows paths, registry/toolchain checks, or Tauri/Windows bundling details.
- When PowerShell is used for text output, follow [encoding.md](encoding.md). Do not trust Cyrillic output from `Get-Content` without checking the real UTF-8 file content.
- Keep `npm.cmd` for npm scripts in this Windows environment.
- Do not rewrite files only because command output appears garbled in PowerShell.
