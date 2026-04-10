# Plan: Settings Editor Parity

Spec: [../specs/features/settings-editor-parity.md](../specs/features/settings-editor-parity.md)

## Implementation Steps

1. Mirror the index page markdown editor markup in the settings page.
2. Keep the settings-specific form state and validation wiring unchanged.
3. Run frontend verification with `npm run test`, `npm run typecheck`, and
   `npm run build`.
