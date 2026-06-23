# Project Notes

## Icons

Use `lucide-react` icon exports with the `Icon` suffix, for example `CameraIcon`, not `Camera`.

## Before starting any task

- If the task involves **planning or proposing an approach**: Read [docs/agent/planning.md](docs/agent/planning.md) before writing a plan.
- If the task involves **complex codebase decisions or known tricky areas**: Read [docs/agent/development-docs-maintenance.md](docs/agent/development-docs-maintenance.md) and check the relevant page in `docs/development/` before re-analysing the problem.

## Before writing or editing code

You MUST Read the relevant rules file before touching related code. Do not rely on memory or assumptions.

- Before creating or editing **any component**: Read [docs/agent/component-structure.md](docs/agent/component-structure.md).
- Before creating or moving **any file or folder**: Read [docs/agent/file-structure.md](docs/agent/file-structure.md).
- Before writing or editing **any `.scss` file**: Read [docs/agent/scss-style-rules.md](docs/agent/scss-style-rules.md).
- Before writing or editing **any `.md` file**: Read [docs/agent/markdown-style-rules.md](docs/agent/markdown-style-rules.md).
- Before running **any shell command**: Read [docs/agent/command-execution.md](docs/agent/command-execution.md).
- Before reading or writing **any file with Cyrillic text**: Read [docs/agent/encoding.md](docs/agent/encoding.md).
- Before implementing a change that **adds, removes, or alters user-visible behavior**: Read [docs/agent/functional-spec-maintenance.md](docs/agent/functional-spec-maintenance.md) and compare the change against `docs/functional-spec/index.md`.

## After finishing changes

After every task that touches code, styles, config, or documentation, you MUST:

1. Read [docs/agent/after-work-checks.md](docs/agent/after-work-checks.md) and run every check listed there.
2. Report which checks were run and whether anything failed or was skipped.

Do not consider the task complete until the checks have been run.

## For visual, layout, or theme changes

After any change to layout, styles, Ant Design tokens, or modal/navigation structure, you MUST:

1. Read [docs/agent/screenshot-testing.md](docs/agent/screenshot-testing.md).
2. Follow the full workflow and checklist described there.
