# Plan: Bootstrap GET Auth Redirect Handling

## References
- Feature spec:
  [../specs/features/bootstrap-get-auth-redirect-handling.md](../specs/features/bootstrap-get-auth-redirect-handling.md)

## Objective
Make React bootstrap GET requests resilient to expired-session redirects by
teaching the shared JSON fetch helper to detect redirected HTML responses and
send the browser to login before any JSON parsing occurs.

## Fixed Implementation Decisions
- The fix WILL stay in `frontend/src/lib/api.ts`.
- The existing `handleAuthRedirectResponse` and content-type detection helpers
  WILL be reused rather than introducing a parallel mechanism.
- Regression coverage WILL be added as a Vitest test under `frontend/src/lib/`.
- No backend or route changes are required.

## Steps
1. Update `fetchJson` to apply the auth-redirect guard after a successful
   response is received and before `response.json()` is called.
2. Add a test that simulates a redirected HTML response from an `/api/v1/`
   bootstrap request and verifies browser navigation is triggered.
3. Run targeted frontend verification (`npm test` for the new test file and
   `npm run typecheck`) to confirm the helper change is safe.

## Verification
- Redirected HTML bootstrap response no longer causes a JSON parse failure.
- Existing JSON bootstrap responses still parse successfully.
- Frontend tests and type checking pass for the changed code.
