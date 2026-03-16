# Current State And Gaps

This summary was prepared from the current codebase on 2026-03-16.

## What Is Already Strong

- Capability-oriented structure exists (`iam`, `notifications`, `background`, shared lib).
- Multitenancy primitives exist through organisation + user mapping + role (`admin`, `member`).
- Public/private API split and middleware boundary are already in place.
- Background queues, workers, and scheduler are implemented with DB-backed jobs.
- Docker and local infra are available.
- Migrations and CI workflow are present.

## Observed Gaps (High Signal)

1. **Metadata mismatch for open-source posture**
   - `Cargo.toml` still describes the package as proprietary.

2. **Invite flow success path appears incorrect**
   - `organisation_service::invite_user` currently ends with `Ok(CommonResponse::Conflict)` even after invite email dispatch.

3. **Invite token verification path likely broken**
   - `verify_invite_link_token` resolves `ORG_INVITE_SECRET` value, then passes that value as the env-key argument to `jwt_verify`.

4. **Invite token duration comment and value are inconsistent**
   - Comment says 7 days, value passed is `7 * 24 * 60` (seconds in this crypto helper), which is ~2h48m.

5. **Potential mapping lookup bug in invite acceptance**
   - `accept_invite` calls `get_mapping_by_id(db, mapping.org_id)` which looks like org-id used where mapping-id is expected.

6. **Potential parameter order bug in register side-effect handler**
   - `add_user_to_organisation` signature is `(org_id, user_id, ...)`, but caller appears to pass `(user.id, org.id, ...)`.

7. **Password whitespace validation regex appears incorrect for Rust**
   - Pattern `"/[\\s\\n]/g"` likely does not enforce intended behavior.

8. **Some tests appear stale versus route wiring**
   - `src/test_utils.rs` references `/auth/login_test/...` route not visible in current controllers.
   - `src/base/e2e_test.rs` calls `/api/v1/health` and `/api/v1/ping` while public APIs mount at `/api/v1/public`.

9. **README API docs path appears outdated**
   - README states `/docs`; runtime currently exposes swagger at `/swagger` and `/private/swagger`.

## Practical Setup Improvements

### Phase 1: Foundation Hygiene

- Align metadata and docs (open source message, swagger paths, env variable list).
- Fix invite flow response semantics and token handling bugs.
- Fix stale tests and API paths.

### Phase 2: Capability Scaffolding

- Add a capability template skeleton under `src/capabilities/_template`.
- Add a short generator script for new capability folders.

### Phase 3: Enterprise Hardening

- Add policy tests for auth boundaries (public vs private routes).
- Add background job reliability tests (retry, delayed/scheduled behavior).
- Add explicit adapter interfaces for CRM/analytics vendor swap.

## Suggested Next Milestone

Ship a `v0.2` cleanup focused only on correctness + docs alignment before adding new major capabilities.
