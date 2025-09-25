# Routes to Services Refactor Plan

This document describes the tasks required to move business logic out of the Actix route handlers in `src/routes` and into corresponding service modules under `src/services`. The end state should leave each route as a thin wrapper that:

1. Extracts request data (path/query/body/auth/user inputs).
2. Invokes a service layer function with the extracted data and shared dependencies.
3. Maps the returned domain/service result into an `HttpResponse`.

All repository access, validation beyond simple extraction failures, and domain-specific branching should live in the service layer.

## Cross-cutting tasks

- [x] Audit each service module (`src/services/*.rs`) and create/expand public APIs that encapsulate the logic currently embedded in the matching route module.
- [x] Standardize on returning `pushkind_common::services::errors::{ServiceError, ServiceResult}` directly from every service API so routes can convert them into HTTP/flash responses.
- [ ] Centralize role/permission checks into reusable helpers (either in services or a shared guard) to eliminate repeated `ensure_role` calls in routes.
- [x] Ensure each service function receives the minimal required inputs (e.g., repository traits, configurations, ZMQ sender, Tera reference) rather than whole Actix extractors.
- [x] Update the route handlers to delegate to the new service functions and translate their outputs into responses/flash messages.
- [ ] Write/adjust unit and integration tests to cover the new service interfaces and the simplified routes.
- [x] Run `cargo fmt`, `cargo clippy --all-features --tests -- -Dwarnings`, and `cargo test --all-features --verbose` to verify the refactor.

## Module-specific task breakdown

### `main`

Routes affected: `index`, `send_email`, `delete_email`, `resend_email`, `track_email`, `export_email_recipients`.

- [x] Implement a `MainService` API that:
  - Provides a function (e.g., `build_index_page`) responsible for loading recipients, groups, emails, custom fields, and retry data, returning a view model/context struct.
  - Handles email creation (`queue_new_email`), deletion (`delete_email`), resend (`queue_email_retry`), tracking updates (`mark_email_opened`), and recipient export (`export_email_recipients`).
  - Encapsulates CSV generation for exports and ZMQ message construction.
- [x] Refactor `SendEmailForm::to_new_email` interactions to live in the service, keeping the route responsible only for extracting and validating the multipart form.
- [x] Move all repository calls (`list_*`, `get_*`, `delete_*`, `update_*`) into the service and expose structured results for the routes to translate into responses.
- [x] Decide on service return types (e.g., domain DTOs or lightweight structs) that the routes can use to render templates via `render_template`.

### `groups`

Routes affected: `groups_show`, `groups_add`, `groups_delete`, `groups_assign`, `groups_modal`.

- [x] Create a `GroupsService` that loads group/recipient/custom field data for the index page and modal rendering.
- [x] Move group creation, deletion, and recipient assignment logic (including validation and error handling) into service methods.
- [x] Centralize form validation to the service level, returning specific error variants for invalid payloads versus repository failures.
- [x] Have the service return ready-to-render context data or DTOs, letting the route focus on flash message emission and HTTP redirect decisions.

### `recipients`

Routes affected: `recipients_show`, `recipients_add`, `recipients_delete`, `recipients_clean`, `recipients_upload`, `recipients_modal`, `recipients_save`, `recipients_source`.

- [x] Build a `RecipientsService` that abstracts recipient search/list pagination, CRUD operations, bulk upload parsing, CSV parsing, and CRM source ingestion.
- [x] Shift responsibility for calling `repo` methods and constructing `Paginated` wrappers into the service.
- [x] Consolidate repeated `redirect("/recipients")` branches by having the service communicate success/failure outcomes that the route translates into redirects or HTTP errors.
- [x] Ensure the service handles multipart parsing, HTML form deserialization, and cookie-driven source loading, surfacing meaningful error messages for the route to flash.
- [x] Provide helper functions to prepare modal context data (recipient details and available groups).

### `settings`

Routes affected: `settings_show`, `unsubscribed_show`, `history_show`, `history_download`, `settings_save`.

- [x] Introduce a `SettingsService` with methods for loading hub configuration, unsubscribed lists, history lists, CSV exports, and hub persistence.
- [x] Move hub initialization (creating if missing) and updates into the service, returning domain DTOs or error variants.
- [x] Encapsulate CSV export logic for history download and shared code for listing recipients/hub state.
- [x] Ensure the service mediates repository access and returns presentation models that the routes pass to `render_template`.

## Deliverables

- Service modules contain the domain and repository logic currently located in the route handlers.
- Routes are simplified to parameter extraction, invoking service functions, handling service errors, and returning responses.
- Comprehensive tests exist for the service layer and route wrappers.
