# React Frontend Migration Preserving Existing Emailer UI

## Status
Stable

## Date
2026-03-27

## Summary
Migrate the current Tera-based `pushkind-emailer` frontend to React-managed UI
components while preserving the existing route structure, Bootstrap styling,
Russian copy, and backend-owned business rules. The migration MUST follow the
same stable pattern already used in `pushkind-auth` and `pushkind-crm`:
server-routed pages,
Vite-built static frontend documents for React-owned pages,
typed client data APIs under `/api/v1/`,
and structured JSON mutation responses with form-owned validation copy.

`pushkind-emailer` MUST NOT become a SPA.

## Problem
The current frontend is split across Tera templates, inline JavaScript,
Bootstrap lifecycle code, and HTML partial/modal rendering. That makes the UI
harder to compose, test, and evolve as recipient management, group assignment,
settings, and history flows grow more interactive.

## Goals
- Introduce React as the component model for user-facing emailer pages.
- Preserve the current route URLs, Bootstrap-based layout, semantics, and
  Russian user-visible copy.
- Preserve current backend validation, authorization, sanitization, queueing,
  and persistence rules.
- Replace template-owned interactive behavior with React-owned components and
  typed data contracts as pages are migrated.
- Align `pushkind-emailer` with the React/Vite migration pattern already in use
  in `pushkind-auth` and `pushkind-crm`.

## Non-Goals
- Introducing client-side routing.
- Redesigning the UI or replacing Bootstrap.
- Moving validation, authorization, sanitization, or persistence rules into the
  browser.
- Replacing the auth/session model with browser token storage.
- Changing email delivery semantics, tracking semantics, or core repository
  rules beyond what React needs for parity.

## In Scope
- The authenticated index page at `GET /`, including retry-prefill behavior.
- Recipient management at `GET /recipients` and related modal/edit flows.
- Group management at `GET /groups` and related modal/assignment flows.
- Admin settings at `GET /settings`.
- Unsubscribed list at `GET /unsubscribed`.
- Delivery history at `GET /history`.
- Shared shell concerns currently handled by Tera layout/navigation.
- Frontend asset build and delivery needed to run React in production and local
  development.

## Out Of Scope
- Anonymous tracking pixel behavior at `GET /track/{email_recipient_id}`.
- Core queueing, delivery, unsubscribe, repository, or schema changes.
- Public third-party API design.

## Functional Requirements

### 1. Rendering Model
- The application MUST keep the existing server-owned route model.
- The application MUST NOT introduce client-side routing for `/`, `/recipients`,
  `/groups`, `/settings`, `/unsubscribed`, or `/history`.
- React MUST be introduced as page-level or island-level components mounted on
  the existing URLs.
- The target state for migrated pages MUST be React-owned page markup served
  from Vite-built static HTML documents after backend access checks.

### 2. Frontend Document Ownership
- React-owned full pages SHOULD be authored in the frontend workspace and built
  by Vite into static HTML documents under `assets/dist/`.
- Rust MUST continue to own authentication and authorization checks before
  serving those documents.
- Page initialization data MUST NOT remain embedded into server-generated HTML
  in the target state.
- Tera MAY remain only as a temporary migration wrapper until a page is fully
  React-backed.

### 3. Markup And Style Preservation
- Migrated React components MUST preserve the current Bootstrap-based layout,
  form structure, modal structure, navigation hierarchy, and class conventions
  unless a deviation is explicitly documented.
- User-visible Russian copy SHOULD remain unchanged except for bug fixes or
  accessibility improvements.
- Existing Bootstrap JS behaviors such as dropdowns and modals MUST continue to
  work.

### 4. Page And Interaction Parity
- `GET /` MUST continue to present the email composer, retry-prefill flow,
  recent emails, recipient/group selectors, and send flow.
- `GET /recipients` MUST continue to present recipients list, add/upload/import
  flows, and recipient edit modal behavior.
- `GET /groups` MUST continue to present groups list, create/delete flows, and
  group membership assignment behavior.
- `GET /settings` MUST continue to present hub SMTP/IMAP/template settings.
- `GET /unsubscribed` MUST continue to present the unsubscribed list.
- `GET /history` MUST continue to present delivery history and export access.
- History and recipient exports MAY remain native download endpoints rather than
  React-owned JSON flows.

### 5. Client Data API Model
- React-owned page initialization MUST prefer typed GET APIs under `/api/v1/`
  rather than HTML-embedded bootstrap payloads or HTML partial rendering.
- DTOs exposed to React MUST be UI-ready and MUST NOT leak raw domain internals
  unnecessarily.
- The target state SHOULD prefer reusable resource-style APIs over one-off
  page-shaped bootstrap payloads where practical.
- Shared shell data such as current user, home URL, navigation, and auth-driven
  user-menu items SHOULD be exposed through a typed shell API.

### 6. Mutation And Validation Semantics
- React-owned mutation flows SHOULD use structured JSON success/error responses
  instead of flash-message-driven redirects or plain-text bodies.
- Field-level validation errors MUST be addressable so React can render them
  inline.
- Validation copy for React-owned forms MUST be owned by `src/forms`, following
  the same pattern used in `pushkind-auth` and `pushkind-crm`.
- Russian validation strings MUST be defined directly on form field
  `#[validate(..., message = "...")]` annotations and on `#[error("...")]`
  annotations for `FormError` enum variants, rather than assembled in routes or
  services.
- Routes SHOULD convert `Form -> Payload` at the boundary before calling
  services, so services can continue using the common `ServiceError` pattern.
- Redirect-based or download-based endpoints MAY remain where they are still the
  correct transport.

### 7. Backend Boundary
- Authorization, validation, sanitization, queueing, and persistence MUST
  remain in Rust services and repositories.
- Routes MUST expose typed DTOs or page-model payloads to React rather than
  leaking template contexts directly.
- Legacy HTML partial/modal endpoints SHOULD be replaced by typed JSON data
  APIs before the corresponding interaction is considered fully migrated.

### 8. Shared Navigation And User Menu
- The top navigation SHOULD follow the same reusable React pattern already used
  in `pushkind-auth` and `pushkind-crm`.
- The user dropdown MUST always include `Домой` and logout.
- Additional menu items SHOULD come from the auth menu API.
- Failure to load auth-driven menu items MUST NOT make `pushkind-emailer`
  unavailable.

### 9. Frontend Tooling
- The repository MUST gain a supported frontend toolchain for React and
  TypeScript source code.
- Production builds MUST emit versioned static assets and any required static
  HTML documents that can be served by the Rust application.
- The server MUST serve the compiled frontend assets directly.
- Local development MUST support efficient frontend iteration without manual
  asset copying.

## Migration Requirements
- The migration MUST be incremental.
- The migration SHOULD converge on the same stable shape used in
  `pushkind-auth` and `pushkind-crm`:
  Vite-built static HTML for React-owned full pages,
  typed `/api/v1/...` client data APIs,
  structured JSON mutation responses,
  and form-owned validation messages.
- Shared React shell components SHOULD be introduced early for navigation,
  user-menu behavior, and common mutation handling.
- Tera MUST be removable as a runtime dependency once all migrated pages are
  fully React-owned.
- Inline JavaScript and template-owned interaction code SHOULD be removed only
  after equivalent React behavior is verified.
- Regression verification SHOULD rely on backend contract tests, frontend
  component or integration tests, and targeted manual checks for
  authentication-dependent flows.

## Acceptance Criteria
- The same URLs continue to serve the corresponding emailer pages and actions.
- Visual appearance remains substantially unchanged for navigation, composer,
  recipients, groups, settings, unsubscribed, and history pages.
- React-owned pages are served from Vite-built frontend documents after backend
  access checks.
- Page data comes from typed client data APIs rather than HTML-embedded
  bootstrap payloads.
- React-owned mutations return structured success/error responses with
  field-addressable validation errors.
- The shared user dropdown behaves consistently with `pushkind-auth` and
  `pushkind-crm`.
- No backend business rule is moved to the client.
- `tera` and `actix-web-flash-messages` are removed from direct
  `pushkind-emailer` dependencies once the migration is complete.
- The React frontend builds reproducibly and its assets are served by the
  application runtime.
- Regression coverage exists for backend page-data contracts and critical
  frontend behavior.

## Risks
- React markup can drift from the current templates unless parity is checked
  explicitly.
- Moving away from plain-text and flash-message mutation flows requires careful
  route-by-route API contract updates.
- Recipient/group modal flows may require new typed APIs before Tera/HTML modal
  endpoints can be removed.
