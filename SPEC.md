# pushkind-emailer Specification

## Summary

pushkind-emailer is a hub-facing web application for orchestrating outbound email
campaigns. Authenticated hub members curate recipients and groups, compose
messages (optionally with attachments), and queue deliveries through a shared
ZeroMQ pipeline. The service also tracks opens, manages unsubscribes, and
exports delivery history.

## Goals

- Provide a hub-scoped UI for composing, sending, and retrying outbound emails.
- Maintain recipient and group directories with validation and bulk import.
- Enforce authorization via Pushkind auth service roles.
- Keep business logic in services and persistence in repository layer.
- Support auditing via CSV exports and history views.

## Non-Goals

- Direct email delivery (handled by downstream worker pipeline).
- Public API for third-party integrations (this service is hub-internal).
- Rich WYSIWYG editing (message body is sanitized HTML).

## Actors and Roles

- Hub user with `SERVICE_ACCESS_ROLE` (`emailer`) for standard workflows.
- Hub admin with `SERVICE_ADMIN_ROLE` (`admin`) for settings changes.
- Anonymous user only hits the open-tracking pixel endpoint.

## Core User Flows

### Compose and Send Email

1. Load `/` to view recent emails, recipients, groups, and custom fields.
2. Compose a message, optional subject, optional attachment (10 MB limit).
3. Select recipients by individual email or group id list.
4. Service sanitizes HTML, deduplicates recipients, skips unsubscribed, and
   optionally applies cooldown days to filter recent recipients.
5. Queue message via ZeroMQ.

### Retry / Delete Email

- Retry: enqueue a retry command for an existing email id.
- Prefill: `GET /?retry={email_id}` loads an existing email into the composer (copy/edit/send).
- Delete: remove an email and its associated snapshot data.

### Track Opens

- Pixel endpoint `/track/{email_recipient_id}` marks `opened=true` on the
  email recipient snapshot (`email_recipients.id`) and returns a placeholder image.

### Manage Recipients

- Add/edit/delete recipients with validation and custom fields.
- Bulk upload from CSV (header row required with `name` and `email`).
- Import from external source URL using the `id` cookie for auth.
- Clean removes all recipients and groups for the hub.

### Manage Groups

- Create/delete groups for the hub.
- Assign recipients to groups (overwrites existing membership).
- Load group modal with membership list.

### Hub Settings and History

- View/update hub SMTP/IMAP settings and email template.
- View unsubscribed recipients.
- View and export delivery history as CSV.

## HTTP Routes (Actix)

- `GET /` index page and email composer.
- `GET /?retry={email_id}` prefill composer from an existing email.
- `POST /email/send` queue a new email.
- `POST /email/{email_id}/delete` delete an email.
- `POST /email/{email_id}/resend` retry an email.
- `GET /track/{email_recipient_id}` open tracking pixel.
- `GET /email/{email_id}/recipients/export` export recipients CSV.
- `GET /recipients` recipients list.
- `POST /recipient/add` add a recipient.
- `POST /recipient/{recipient_id}/delete` delete a recipient.
- `POST /recipients/clean` delete all recipients and groups for hub.
- `POST /recipients/upload` upload CSV of recipients.
- `POST /recipients/source` import recipients from external source.
- `POST /recipient/{recipient_id}/modal` load recipient modal.
- `POST /recipient/{recipient_id}/save` update recipient.
- `GET /groups` group overview.
- `POST /group/add` create group.
- `POST /group/{group_id}/delete` delete group.
- `POST /group/{group_id}/assign` assign recipients to group.
- `POST /group/{group_id}/modal` load group modal.
- `GET /settings` hub settings page.
- `POST /settings/save` update hub settings.
- `GET /unsubscribed` view unsubscribed list.
- `GET /history` view delivery history.
- `GET /history/download` export history CSV.

## React Client Data APIs

React-owned pages fetch typed JSON under `/api/v1/` using resource-style
endpoints rather than page-named bootstrap routes.

- `GET /api/v1/iam` shared shell data for current user, navigation, and menu items.
- `GET /api/v1/emails` email collection data for the index page, including retry-prefill,
  recipient/group options, custom fields, and paginated recent emails.
- `GET /api/v1/recipients` recipient collection data.
- `GET /api/v1/recipients/{recipient_id}` recipient details for the edit modal.
- `GET /api/v1/groups` group collection data.
- `GET /api/v1/groups/{group_id}` group details for the assignment modal.
- `GET /api/v1/hub-settings` hub SMTP/IMAP/template settings data.
- `GET /api/v1/unsubscribed-recipients` unsubscribed recipient collection data.
- `GET /api/v1/email-history` email history collection data.
- `GET /api/v1/no-access` no-access page data.

## Invariants

- Recipient email addresses are unique per hub.
- A recipient may belong to zero or more groups.
- Unsubscribes are keyed by (email_address, hub_id).
- Unsubscribed recipients MUST NOT receive new emails.
- Email recipient snapshots are immutable after creation (except open/reply status).
- Open tracking is idempotent per email recipient snapshot.
- Group membership updates are replace-all for the target group.

## Authorization Rules

- `/track/{email_recipient_id}` bypasses auth and must be reachable anonymously.
- All other routes require a hub-scoped authenticated user.
- `GET /settings` and `POST /settings/save` require admin role.
- All other authenticated routes require emailer role.
- Destructive operations (`/recipients/clean`, `/group/{id}/delete`, `/recipient/{id}/delete`,
  `/email/{id}/delete`) do not require elevated roles beyond emailer access.

## HTTP Error Semantics

HTML form routes use flash messages and redirects for success/error states,
except `/email/send`, which returns `200` with a plain-text body for both success
and validation failures (AJAX pattern).

| Route | Success | Validation error | Unauthorized | Not found | Other errors |
| --- | --- | --- | --- | --- | --- |
| `POST /email/send` | `200` plain-text success message | `200` plain-text validation message | redirect `/na` + flash | n/a | `500` plain-text error |
| `POST /email/{id}/delete` | redirect `/` + flash | n/a | redirect `/na` + flash | redirect `/` + flash | redirect `/` + flash |
| `POST /email/{id}/resend` | redirect `/` + flash | n/a | redirect `/na` + flash | redirect `/` + flash | `500` |
| `POST /recipients/upload` | redirect `/recipients` + flash | redirect `/recipients` + flash | redirect `/na` + flash | n/a | redirect `/recipients` + flash |
| `POST /recipients/source` | redirect `/recipients` + flash | redirect `/recipients` + flash | redirect `/na` + flash | n/a | redirect `/recipients` + flash |

Pixel endpoint semantics:

- `GET /track/{email_recipient_id}` returns a redirect to `/assets/placeholder.png`.
- Response headers do not set explicit cache control; clients may cache per default behavior.
- `404` for unknown email recipient id, `500` for unexpected failures.

## Data Model (SQLite via Diesel)

- `hubs`: hub SMTP/IMAP settings, template, last IMAP UID.
- `recipients`: recipient identity plus a denormalized `fields` string used for search.
- `recipient_fields`: normalized custom fields (key/value per recipient).
- `groups`: recipient group metadata (hub-scoped).
- `groups_recipients`: join table for group membership.
- `emails`: outbound email metadata, attachment info, counters.
- `email_recipients`: snapshot of recipients per email (opened/sent/replied + name + JSON fields).
- `unsubscribes`: email_address per hub with optional reason.
- `recipient_fts*`: FTS tables for search.

## Architecture

- `src/domain`: typed value objects and core entities.
- `src/repository`: traits and Diesel implementation for persistence.
- `src/services`: business logic and orchestration.
- `src/dto`: data structures for template rendering.
- `src/forms`: input validation and conversion into domain types.
- `src/routes`: Actix handlers that map HTTP to services.
- `templates/`: Tera templates with sanitized inputs.

Services accept repository traits (`GroupReader`, `RecipientWriter`, etc.) to
allow unit testing with mock repositories. Repositories translate Diesel models
to domain types via `From` implementations.

## Validation and Sanitization

- Strongly typed domain newtypes enforce invariants (positive IDs, valid emails,
  non-empty strings, valid ports/urls).
- Forms use `validator` for input validation.
- Message body is sanitized with `ammonia`.
- Attachment and CSV uploads limited to 10 MB.

## External Integrations

- Pushkind auth service for authentication and role checks.
- Pushkind CRM for recipient quick links.
- Pushkind files service (configured in `ServerConfig`, not used by the web UI today).
- ZeroMQ to enqueue email delivery and retry commands.

## Error Handling

- Repository methods return `RepositoryResult<T>` with typed errors.
- Service layer returns `ServiceResult<T>` and maps errors for handlers.
- Handlers surface errors via flash messages and redirects or HTTP status codes.

## Data Lifecycle

- Emails are persisted with recipient snapshots at send time.
- Recipient snapshots persist for history and exports; only open/reply flags may change.
- Deleting an email removes its recipient snapshots and therefore removes history for that email.
- Unsubscribes are persisted per (email_address, hub_id) and block future sends for that hub.
- Open tracking can be called multiple times without changing correctness (idempotent).

Audit history is derived from the `email_recipients` snapshot table and exposed
via the history page and CSV export; there is no separate append-only audit log.
History exports are generated on demand from snapshots, so deleted emails are
not included in future exports.

## Observability

- Errors are logged via `log` with contextual messages.
