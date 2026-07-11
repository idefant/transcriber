# Project Notes

## Language

Общайся с пользователем и строй планы исключительно на русском языке.

## Icons

Use `lucide-react` icon exports with the `Icon` suffix, for example `CameraIcon`, not `Camera`.

## Utilities

Do not hand-roll a helper when `lodash-es` already provides it. Import named functions from the package root:

```ts
import { clamp, debounce } from 'lodash-es';
```

- Before writing a helper, check whether `lodash-es` covers it, for example `clamp`, `debounce`, `throttle`, `partition`, `groupBy`, `keyBy`, `uniqBy`, `orderBy`, `isEqual`, `cloneDeep`, `pick`, `omit`, `chunk`, `range`, `takeRight`, `dropRight`.
- When `lodash-es` has no equivalent, put the helper in `src/shared/utils` and re-export it from that barrel instead of redefining it next to a component. It already provides `mod` (Euclidean modulo) and `rotate` (cyclic array shift), which lodash lacks.
- Reusable hooks live in `src/shared/hooks`. Prefer `useDebouncedCallback` over hand-written `setTimeout`/`clearTimeout` plumbing in a component; it keeps the debounced instance stable across renders and returns `run`, `cancel`, and `flush`.
- Never import the CommonJS `lodash` package, a default export (`import _ from 'lodash-es'`), or per-method paths such as `lodash-es/clamp`. Only `import { func } from 'lodash-es';` keeps the Vite ESM build tree-shakeable.
- Leave already-simple native code alone. Do not rewrite `Math.floor(x)`, `Math.max(a, b)`, `arr.map(...)`, `arr.length === 0`, or `Object.keys(o)` into `floor`, `max`, `map`, `isEmpty`, or `keys`.
- Reach for `lodash-es` when the native version needs a named helper, a manual loop, or several chained steps to express one idea. Replacing a one-line native call with a lodash call is not an improvement.
- Iteratee-based helpers (`partition`, `groupBy`, `keyBy`, `sortBy`, `orderBy`, `uniqBy`) drag in lodash's `baseIteratee` graph, which costs roughly 15 kB in the bundle. Use them when they replace real logic, not when they merely restate a single `filter` or `map`.

## Comments

Document exported symbols with JSDoc block comments, not `//`. Editors surface a JSDoc docstring on hover and in autocomplete, while a line comment above an export is invisible at the call site:

```ts
/** Runs a waiting call right now instead of waiting out the delay. Does nothing when idle. */
flush: () => void;
```

- Use `//` for explanations inside a function body and above non-exported internals.
- Do not restate the signature. Document what the caller cannot see: edge cases, units, invariants, and the reason the code exists.
- Add `@example` when the behavior is easier to show than to describe, for example a return value that is not obvious from the name.

## Tests

Do not write or add tests unless the user explicitly asks for tests.
Do not proactively suggest tests unless they are explicitly required by the task or the risk/complexity is high enough that skipping tests would be unreasonable.

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
