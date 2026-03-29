# Plan: React Frontend Migration

## References
- Feature spec:
  [../specs/features/react-frontend-migration.md](../specs/features/react-frontend-migration.md)

## Objective
Introduce React for the `pushkind-emailer` frontend while preserving the
current route structure, Bootstrap styling, Russian copy, and backend-owned
emailer business rules. The migration remains server-routed and non-SPA, and
converges on:
Vite-built static HTML for React-owned full pages,
typed `/api/v1/...` client data APIs,
structured JSON mutation responses,
and form-owned validation copy for React-owned flows.

## Fixed Implementation Decisions
- Frontend source code WILL live in `frontend/`.
- Production frontend build output WILL live in `assets/dist/`.
- The React toolchain WILL use `npm`, React, TypeScript, and Vite.
- The backend WILL continue to own routing, authentication, authorization,
  validation, sanitization, queueing, redirects where still applicable, and
  persistence.
- The application server WILL continue to serve compiled frontend assets from
  the existing `/assets` path.
- Vite WILL own the static HTML documents for React-owned full-page routes.
- React page initialization WILL fetch typed JSON data from backend endpoints;
  page data WILL NOT remain embedded into server-generated HTML in the target
  state.
- New GET endpoints introduced for React-owned page data WILL be versioned
  under `/api/v1/`.
- Those GET endpoints MUST prefer reusable resource-style contracts over
  page-shaped bootstrap endpoints.
- Validation copy for React-owned forms WILL live in `src/forms`.
- Russian validation strings WILL be defined directly in
  `#[validate(..., message = "...")]` annotations on form fields and in
  `#[error("...")]` annotations on `FormError` enum variants, following the
  same ownership pattern used in `pushkind-auth`.
- Routes SHOULD convert `Form -> Payload` at the boundary before calling
  services, following the same pattern used in `pushkind-auth` and
  `pushkind-crm`.
- The top navigation user dropdown WILL align with the reusable auth/CRM
  pattern and hydrate menu items from the auth menu API without blocking the
  initial page render.
- Tera WILL be used only as a temporary migration wrapper and MUST be removable
  from runtime paths once page migration is complete.
- `tera` and `actix-web-flash-messages` MUST be removed from direct
  `pushkind-emailer` dependencies by the end of the migration.
- Regression verification WILL rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Repository Layout
The implementation SHOULD create and use the following structure:

```text
frontend/
  package.json
  package-lock.json
  tsconfig.json
  vite.config.ts
  src/
    entries/
    components/
    pages/
    styles/
    lib/
assets/
  dist/
src/
  dto/
  routes/
  services/
  forms/
templates/
```

Directory intent:
- `frontend/src/entries/`:
  entrypoints for full-page emailer routes.
- `frontend/src/components/`:
  reusable shell, navbar, user-menu, form, modal, list, and table components.
- `frontend/src/pages/`:
  page-level React components for composer, recipients, groups, settings,
  unsubscribed, and history views.
- `frontend/src/lib/`:
  typed payload readers, API clients, endpoint builders, Bootstrap adapters,
  and cross-service menu helpers.
- `frontend/src/styles/`:
  CSS imports preserving the current Bootstrap-based output.
- `assets/dist/`:
  compiled JavaScript, CSS, static HTML, and manifest output.

## Toolchain And Build Outputs

### Frontend Package Management
- Use `npm` as the package manager.
- Commit `frontend/package-lock.json`.
- Do not introduce `pnpm`, `yarn`, or an alternative JavaScript runtime.

### Build Tool
- Use Vite to build the React frontend.
- Configure Vite to emit compiled assets into `assets/dist/`.
- Configure Vite to emit a manifest file at `assets/dist/manifest.json`.
- Configure explicit entrypoints for the emailer full-page routes that are
  migrated to React.

### Required `package.json` Scripts
The frontend package MUST expose at least these scripts:
- `dev`
- `build`
- `preview`
- `test`
- `lint`
- `typecheck`
- `format`

### Source Control Hygiene
- Add `frontend/node_modules/` to `.gitignore`.
- Add `assets/dist/` to `.gitignore` unless deployment later requires committed
  build artifacts.

## Backend Integration

### Asset Serving
- Keep Actix static serving for `/assets` and ensure it covers `assets/dist/`.

### Built HTML Serving
- Add a backend helper that serves the built Vite HTML entry for each
  React-owned full-page route after authentication and authorization checks.
- Align the helper with the thin frontend-loading pattern already used in
  `pushkind-auth`, `pushkind-files`, and `pushkind-crm`.
- Rust MUST stop assembling full-page HTML at request time once a route has
  been fully migrated.

### Client Data APIs
- Add typed DTOs under `src/dto/` for reusable emailer client data APIs.
- Prefer typed GET endpoints under `/api/v1/` over HTML-embedded bootstrap data
  or HTML partial rendering.
- Prefer resource-style endpoints over page-named bootstrap endpoints for
  composer, recipients, groups, settings, unsubscribed, and history data.
- The initial DTO surface SHOULD cover:
  current-user/session and shell data,
  composer page data,
  recipients list and recipient modal data,
  groups list and group modal data,
  settings data,
  unsubscribed list data,
  history list data,
  and supporting select-option or lookup datasets required by React forms.
- Existing export and tracking endpoints MAY remain as direct HTTP download or
  anonymous endpoints where that transport is still correct.

### Structured Mutation Responses
- Introduce auth/CRM-style JSON mutation response DTOs for React-owned emailer
  interactions.
- Field errors SHOULD use a stable field-addressable shape.
- Validation copy MUST come from `src/forms`.
- Form validation messages MUST be authored on validator macro annotations and
  `thiserror` enum annotations in the form layer rather than composed in routes
  or services.
- The plain-text `POST /email/send` response model MUST be replaced once the
  composer is React-owned.

### Shared Navigation
- Add a shared React shell for navbar, layout wiring, user-menu behavior, and
  common mutation handling.
- The shell SHOULD align with the reusable dropdown/menu approach already used
  in `pushkind-auth` and `pushkind-crm`.

## Frontend Runtime Requirements

### Bootstrap Integration
- Keep Bootstrap CSS and Bootstrap Icons in the rendered output.
- Preserve Bootstrap JS behavior for dropdowns and modals.
- Move inline Bootstrap lifecycle code into React-safe helpers under
  `frontend/src/lib/`.

### Data Loading
- React-owned full pages MUST fetch typed JSON data after the static HTML
  document loads.
- The frontend SHOULD use shared API helpers that compose page state from
  narrower resource endpoints.
- Auth menu loading MUST happen after the main emailer page data is ready so
  auth slowness does not blank the page.
- React MUST render explicit fatal error states for required data failures.

### Form And Action Handling
- React-owned mutation flows SHOULD use structured JSON request/response
  handling instead of flash-message-driven redirects or plain-text bodies.
- Native form submission MAY remain for interactions that have not yet been
  migrated or for direct download endpoints.
- Recipient and group modal behavior SHOULD move from template fragments to
  typed React components and typed data APIs.
- Composer, retry, recipient import/upload, and settings save flows SHOULD be
  migrated to explicit frontend API helpers.

## Migration Sequence

### Phase 1: Foundation
Deliverables:
- `frontend/` directory with React, TypeScript, and Vite configured.
- Build output emitted to `assets/dist/`.
- Backend helpers for serving built frontend HTML documents.
- Developer documentation for installing Node and building frontend assets.

Exit criteria:
- `npm run build` succeeds.
- The server can serve one Vite-built frontend document and load its compiled
  assets.

### Phase 2: Shared Shell And Navigation
Deliverables:
- Shared React shell for navbar, common layout wiring, and Bootstrap lifecycle
  integration.
- Reusable React user dropdown aligned with auth/CRM.
- Auth menu hydration after initial page render, with resilient fallback to the
  always-present `Домой` link and logout action.

Exit criteria:
- Shared shell behavior no longer depends on inline JavaScript in the base
  template.
- A React-owned navbar/user-menu can render without blocking on auth menu data.

### Phase 3: Full-Page Document Serving And Shell/Page Data APIs
Deliverables:
- Vite-managed HTML entries for early-migration emailer pages.
- Typed `/api/v1/...` shell and page-data endpoints.
- Typed frontend payload readers and API clients.

Exit criteria:
- At least one emailer page can be served from a Vite-built HTML document and
  initialize entirely from typed client data APIs.

### Phase 4: Composer And Index Migration
Deliverables:
- React-backed `GET /` page preserving composer behavior, retry-prefill,
  recent emails list, recipient/group selectors, and send/delete/resend flows.
- Structured JSON handling for composer-related mutations.
- React replacement for any page-specific inline script behavior on the index
  page.

Exit criteria:
- The index/composer page is React-rendered with visual and behavioral parity.
- `POST /email/send` no longer relies on plain-text success/error bodies.

### Phase 5: Recipients Migration
Deliverables:
- React-backed `GET /recipients` page preserving list, add, delete, upload,
  import-from-source, clean, and edit-modal behavior.
- Typed React replacement for recipient modal HTML rendering.
- Structured JSON handling for React-owned recipient mutations.

Exit criteria:
- The recipients page works end to end through React-owned UI without depending
  on Tera-owned page markup or modal fragments.

### Phase 6: Groups Migration
Deliverables:
- React-backed `GET /groups` page preserving groups list, create/delete flows,
  assignment flow, and group modal behavior.
- Typed React replacement for group modal HTML rendering.
- Structured JSON handling for React-owned group mutations.

Exit criteria:
- The groups page works end to end through React-owned UI without depending on
  Tera modal fragments or page-specific inline behavior.

### Phase 7: Settings, Unsubscribed, And History Migration
Deliverables:
- React-backed `GET /settings` page preserving SMTP/IMAP/template configuration
  and save behavior.
- React-backed `GET /unsubscribed` page preserving list behavior.
- React-backed `GET /history` page preserving history display and download
  access.

Exit criteria:
- Settings, unsubscribed, and history pages are React-rendered with parity.
- Download/export endpoints continue to work correctly alongside the new pages.

### Phase 8: Legacy Frontend Removal
Deliverables:
- Remove obsolete Tera page templates and fragments no longer used for React
  pages.
- Remove inline scripts and template-owned interaction code no longer needed at
  runtime.
- Remove temporary migration wrappers once all targeted pages are React-backed.
- Remove direct `tera` and `actix-web-flash-messages` dependencies from
  `pushkind-emailer`.

Exit criteria:
- No targeted emailer page depends on Tera-owned page markup, flash-message
  middleware, or page-specific inline scripts at runtime.

## Verification Strategy
- Add backend tests for built-HTML route selection, page-data DTOs, and
  structured JSON mutation responses.
- Add frontend unit tests for payload parsing, API clients, Bootstrap helpers,
  and local interactive UI behavior.
- Add frontend component or integration tests for composer, recipients, groups,
  settings, unsubscribed, history, and user-menu behavior.
- Use targeted manual verification for flows coupled to authentication,
  exports/downloads, and external recipient import behavior.

## Required Commands
- `cargo build --all-features --verbose`
- `cargo test --all-features`
- `cargo clippy --all-features --tests -- -Dwarnings`
- `cargo fmt --all -- --check`
- `cd frontend && npm run typecheck`
- `cd frontend && npm run test`
- `cd frontend && npm run build`
