# pushkind-emailer

`pushkind-emailer` is the hub-facing web application for orchestrating outbound
email across the Pushkind platform. It lets authorised members curate
recipients and groups, compose hub-branded messages, and queue deliveries
through the shared ZeroMQ worker pipeline. The service is implemented in Rust
on top of Actix Web, Diesel, and Tera, and integrates tightly with the shared
`pushkind-common` crate for authentication, configuration, and reusable UI
helpers.

## Features

- **Queue email campaigns** – Compose sanitised messages, attach files up to
  10 MB, and send them through ZeroMQ to the background mailer while skipping
  unsubscribed recipients and duplicate addresses.
- **Recipient directory management** – Browse recipients with pagination, view
  custom fields, and edit records inline while staying scoped to the current
  hub.
- **CSV and remote imports** – Bulk upload recipients from CSV or pull them from
  Pushkind services over authenticated HTTP requests; custom fields and group
  subscriptions are inferred automatically.
- **Group management** – Create, assign, and delete recipient groups, reuse
  custom field definitions, and inspect membership via modal dialogues.
- **Hub settings** – Configure hub metadata, review unsubscribed recipients,
  inspect recent delivery history, and export CSV snapshots for auditing.
- **Delivery insights** – Track opens via a pixel endpoint and export per-email
  recipient status (sent/opened/replied) for downstream analytics.

## Architecture at a Glance

The codebase follows a clean, layered structure so that business logic can be
exercised and tested without going through the web framework:

- **Domain (`src/domain`)** – Type-safe models that describe recipients, groups,
  unsubscribes, and CSV export helpers so business logic can operate without
  touching Diesel structs directly.
- **Repository (`src/repository`)** – Traits that describe the persistence
  contract and a Diesel-backed implementation (`DieselRepository`) that speaks to
  a SQLite database. Each module translates between Diesel models and domain
  types and exposes strongly typed query builders.
- **Services (`src/services`)** – Application use-cases that orchestrate domain
  logic, repository traits, and Pushkind authentication helpers. Services return
  `ServiceResult<T>` and map infrastructure errors into well-defined service
  errors.
- **Forms (`src/forms`)** – `serde`/`validator` powered structs that handle
  request payload validation, CSV parsing, and transformation into domain types.
- **Routes (`src/routes`)** – Actix Web handlers that wire HTTP requests into the
  service layer and render Tera templates or redirect with flash messages.
- **Templates (`templates/`)** – Server-rendered UI built with Tera and
  Bootstrap 5, backed by sanitized HTML rendered via `ammonia` when necessary.

Because the repository traits live in `src/repository/mod.rs`, service functions
accept generic parameters that implement those traits. This makes unit tests easy
by swapping in the `mockall`-based fakes from `src/repository/mock.rs`.

## Technology Stack

- Rust 2024 edition
- [Actix Web](https://actix.rs/) with identity, session, and flash message
  middleware
- [Diesel](https://diesel.rs/) ORM with SQLite and connection pooling via r2d2
- [Tera](https://tera.netlify.app/) templates styled with Bootstrap 5.3
- [`pushkind-common`](https://github.com/pushkindt/pushkind-common) shared crate
  for authentication guards, configuration, database helpers, and reusable
  patterns
- Supporting crates: `chrono`, `validator`, `serde`, `ammonia`, `csv`, and
  `thiserror`

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Configuration

Settings are layered via the [`config`](https://crates.io/crates/config) crate in the following order (later entries override earlier ones):

1. `config/default.yaml` (checked in)
2. `config/{APP_ENV}.yaml` where `APP_ENV` defaults to `local`
3. Environment variables prefixed with `APP_` (loaded automatically from a `.env` file via `dotenvy`)

Key settings you may want to override:

| Environment variable | Description | Default |
| --- | --- | --- |
| `APP_SECRET` | 64-byte secret used to sign cookies and flash messages | _required_ |
| `APP_DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `APP_ADDRESS` | Interface to bind | `127.0.0.1` |
| `APP_PORT` | HTTP port | `80` (override to `8079` in local.yaml) |
| `APP_DOMAIN` | Cookie domain (without protocol) | _required_ |
| `APP_TEMPLATES_DIR` | Glob pattern for templates consumed by Tera | `templates/**/*` |
| `APP_ZMQ_EMAILER_PUB` | ZeroMQ PUB endpoint for outgoing email events | `tcp://127.0.0.1:5557` |
| `APP_ZMQ_EMAILER_SUB` | ZeroMQ PUB endpoint for inbound email events | `tcp://127.0.0.1:5558` |
| `APP_ZMQ_REPLIER_SUB` | ZeroMQ PUB endpoint for inbound client events | `tcp://127.0.0.1:5559` |
| `APP_ZMQ_REPLIER_SUB` | ZeroMQ PUB endpoint for inbound email reply events | `tcp://127.0.0.1:5560` |
| `APP_AUTH_SERVICE_URL` | URL of the Pushkind authentication service | _required_ |
| `APP_CRM_SERVICE_URL` | Base URL of the CRM service used for quick links | _required_ |
| `APP_FILES_SERVICE_URL` | Base URL file storage service used for uploading files | _required_ |

Switch to the production profile with `APP_ENV=prod` or provide your own
`config/{env}.yaml`. Environment variables always win over YAML values, so a
local `.env` file containing `APP_SECRET=<64-byte key>` (generate with
`openssl rand -base64 64`) and any overrides will take effect without changing
the checked-in config files.

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
cargo install diesel_cli --no-default-features --features sqlite # only once
diesel migration run
```

A SQLite file will be created at the location given by `DATABASE_URL`.

## Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:8080` by default and serves static
assets from `./assets` in addition to the Tera-powered HTML pages. Authentication
and authorization are enforced via the Pushkind auth service and the
`SERVICE_ACCESS_ROLE` constant.

## Quality Gates

The project treats formatting, linting, and tests as required gates before
opening a pull request. Use the following commands locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
```

Alternatively, the `make check` target will format the codebase, run clippy, and
execute the test suite in one step.

## Testing

Unit tests exercise the service and form layers directly, while integration
tests live under `tests/`. Repository tests rely on Diesel’s query builders and
should avoid raw SQL strings whenever possible. Use the mock repository module to
isolate services from the database when writing new tests.

## Project Principles

- **Domain-driven**: keep business rules in the domain and service layers and
  translate to/from external representations at the boundaries.
- **Explicit errors**: use `thiserror` to define granular error types and convert
  them into `ServiceError`/`RepositoryError` variants instead of relying on
  `anyhow`.
- **No panics in production paths**: avoid `unwrap`/`expect` in request handlers,
  services, and repositories—propagate errors instead.
- **Security aware**: sanitize any user-supplied HTML using `ammonia`, validate
  inputs with `validator`, and always enforce role checks with
  `pushkind_common::routes::check_role`.
- **Testable**: accept traits rather than concrete types in services and prefer
  dependency injection so the mock repositories can be used in tests.

Following these guidelines will help new functionality slot seamlessly into the
existing architecture and keep the service reliable in production.
