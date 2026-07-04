# Resettable Default-Backed Fields

Some settings fields store an optional user override but still need to show and use a built-in default when the override is absent. Examples in this project are `stt.systemPrompt`, `postProcess.systemPrompt`, and `postProcess.userPromptTemplate`.

## Canonical Stored State

Store these fields as `string | null` on the frontend and `Option<String>` on the Rust side.

- `null` / `None` means "the user has not set an override; use the default value".
- Any string value, including `''`, means "the user explicitly set this value; use it as-is".

Do not add parallel `touched`, `isDefault`, or similar flags for these fields. Whether the field has a user override is derived directly from `value !== null`.

## UI Contract

When the stored value is `null`:

- the settings UI displays the default value;
- the model request uses the default value;
- the reset button is disabled.

When the stored value is a string:

- the settings UI displays that exact string;
- the model request uses that exact string;
- the reset button is enabled.

Reset writes `null` back to storage. It does not write the default text into storage.

## Toggle Contract

Feature toggles such as `useCustomPrompt` / `useCustomPrompts` control only whether the override field is active for model execution. They must not mutate the stored text field when toggled on or off.

This means:

- turning the toggle on does not write the default text into storage;
- turning the toggle off does not clear the stored string;
- if the stored value is `null`, the UI may still display the default while the toggle is on because that is the effective fallback value, not a persisted override.

## Serialization Rule

Persist `None` as JSON `null` for these fields. Avoid omitting the property on save, because the explicit `null` shape makes the default-vs-override contract easier to inspect and reason about in `processing.json`.

For partial update inputs on the Rust side, plain `Option<Option<T>>` is not enough to model this contract. With Serde, a missing field and a present `null` both collapse too easily into the same state. Use an explicit tri-state input wrapper that distinguishes:

- field missing;
- field present with `null`;
- field present with a concrete value.

Without that wrapper, reset-to-default requests can silently become no-ops because the backend cannot tell `null` apart from "the client did not send this field".

## Migration Rule

If an older config version contains extra helper flags for such a field, remove them from the schema and derive the state from the nullable value during normal load/save flow. A dedicated migration is only needed if the old representation cannot be interpreted as `string | null` without ambiguity.
