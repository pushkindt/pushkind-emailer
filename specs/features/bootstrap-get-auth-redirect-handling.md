# Bootstrap GET Auth Redirect Handling

## Status
Stable

## Date
2026-03-29

## Summary
Ensure React bootstrap GET requests under `/api/v1/` handle expired-session
redirects consistently with JSON mutation helpers. When auth middleware
redirects an API request to an HTML login page, the frontend MUST navigate the
browser to that destination instead of attempting JSON parsing and surfacing a
fatal bootstrap error.

## Problem
The shared `fetchJson` helper currently assumes every successful response body
is JSON and unconditionally calls `response.json()`. Because `/api/*` is behind
`RedirectUnauthorized`, an expired session can produce a redirected `200` HTML
response from the auth flow. `fetch` follows that redirect, so the frontend
receives non-JSON content and fails during parsing.

## Goals
- Detect redirected non-JSON responses in bootstrap GET helpers.
- Reuse the same auth-redirect behavior already present in mutation helpers.
- Prevent fatal page bootstrap failures when a session has expired.

## Non-Goals
- Changing backend auth middleware behavior.
- Redesigning API error contracts beyond expired-session redirect handling.
- Introducing SPA-style auth state management.

## Functional Requirements
- The shared GET JSON helper MUST detect `response.redirected` responses whose
  content type is not JSON.
- For those responses, the frontend MUST redirect the browser to
  `response.url`.
- The helper MUST NOT attempt `response.json()` after detecting such a redirect.
- Existing JSON success and non-redirect error handling MUST remain unchanged.

## Acceptance Criteria
- Loading a React-owned page after session expiry redirects the browser to the
  auth/login destination instead of rendering a fatal parse error.
- The shared GET helper and mutation helpers use the same auth-redirect rule.
- Frontend regression coverage exists for redirected HTML bootstrap responses.
