# pushkind-emailer

`pushkind-emailer` is an email service that powers
the Pushkind ecosystem. Built with Rust, Actix Web and Diesel, it handles
creation of arbitrary email recipients as well as loading them from
pushkind-auth and pushkind-crm, and creating email messages.
Additional binaries `send_email` and `check_reply` handle sending emails over SMTP
and checking for replies over IMAP. The `check_reply` binary establishes a
persistent connection to each hub and updates recipients as new replies arrive.

## Features

- Actix Web server with a cookie-based SSO authentication provided
by pushkind-auth
- SQLite database access via Diesel ORM
- REST API endpoints for user and role management
- Tera templates for server-rendered pages

## Running locally

1. Install [Rust](https://www.rust-lang.org/tools/install).
2. Set the required environment variables:
   - `DATABASE_URL` (e.g. `app.db`)
   - `SECRET_KEY` for session encryption
   - `AUTH_SERVICE_URL` for redirects to the auth service
   - `CRM_SERVICE_URL` for the crm service to load client emails from
   - `ZMQ_ADDRESS` for the send_email binary to send emails to
   - Optional: `PORT`, `ADDRESS`, `DOMAIN`
3. Run database migrations with `diesel migration run` (requires `diesel-cli`).
4. Start the server:

```bash
cargo run
```

The service listens on `http://127.0.0.1:8080` by default.

## Testing

Run the test suite with:

```bash
cargo test
```
