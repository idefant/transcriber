# After Work Checks

After code, styles, config, or documentation edits, run the relevant project checks before the final response.

Use `npm` for project scripts:

```bash
npm run typecheck
npm run lint
npm run stylelint
npm run format:check
```

Also run the production build when the change affects application code, routing, Vite/TypeScript configuration, dependencies, or other build-sensitive behavior:

```bash
npm run build
```

Use autofix commands only for mechanical fixes:

```bash
npm run lint:fix
npm run stylelint:fix
npm run format
```

In Windows PowerShell, `npm` can resolve to `npm.ps1`. If execution policy blocks that script, use `npm.cmd` for the same command or run the command from Git Bash/Command Prompt.

Do not rely on Husky or lint-staged as the only verification path. They run on commit, while Codex should verify changes before handing work back.

In the final response, mention which checks were run and whether anything failed or was skipped.
