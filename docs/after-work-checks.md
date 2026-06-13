# After Work Checks

After code, styles, config, or documentation edits, run the relevant project checks before the final response.

Use `npm.cmd` on Windows:

```bash
npm.cmd run typecheck
npm.cmd run lint
npm.cmd run stylelint
npm.cmd run format:check
```

Also run the production build when the change affects application code, routing, Vite/TypeScript configuration, dependencies, or other build-sensitive behavior:

```bash
npm.cmd run build
```

Use autofix commands only for mechanical fixes:

```bash
npm.cmd run lint:fix
npm.cmd run stylelint:fix
npm.cmd run format
```

Do not rely on Husky or lint-staged as the only verification path. They run on commit, while Codex should verify changes before handing work back.

In the final response, mention which checks were run and whether anything failed or was skipped.
