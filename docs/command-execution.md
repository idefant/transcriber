# Command Execution

Prefer Git Bash for project commands when it is available:

```text
C:\Program Files\Git\bin\bash.exe
```

## Rules

- Use Bash for reading/searching files, inspecting text output, and running cross-platform commands.
- Use PowerShell when the command is Windows-specific, for example `npm.cmd`, Windows paths, registry/toolchain checks, or Tauri/Windows bundling details.
- When PowerShell is used for text output, run the command through the UTF-8 wrapper:

  ```cmd
  .\scripts\powershell-utf8.cmd "Get-Content -Raw README.md"
  ```

  From PowerShell, pass the wrapped command in single quotes so `$variables` are not expanded before the wrapper starts:

  ```powershell
  .\scripts\powershell-utf8.cmd 'Get-Content -Raw README.md'
  ```

- Inside that wrapper, `Get-Content` defaults to UTF-8. Without it, do not trust Cyrillic output from `Get-Content`; check the real UTF-8 file content instead.
- Keep `npm.cmd` for npm scripts in this Windows environment.
- Do not rewrite files only because command output appears garbled in PowerShell.
