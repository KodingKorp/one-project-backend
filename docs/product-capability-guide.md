# Product Capability Guide

Use this guide when adding a new business capability so platform defaults remain reusable.

## Goal

A product team should only build `src/capabilities/<product>` and register it in existing extension points.

## Step 1: Create Capability Module

Create a folder:

`src/capabilities/<product>/`

Recommended files:

- `mod.rs`
- `service.rs`
- `objects.rs`
- `controllers/mod.rs`
- `services/mod.rs`
- `repositories/mod.rs`
- `entities/mod.rs`
- `job_handlers/mod.rs` (if background processing is needed)

## Step 2: Export Module

Add the module export in `src/capabilities/mod.rs`.

## Step 3: Implement Service Contract

In `service.rs`, implement `Service` from `src/capabilities/lib/service_trait.rs`:

- `register_routes()` for public endpoints.
- `register_private_routes()` for authenticated endpoints.
- `register_background()` for queue/job/schedule registration.

## Step 4: Register In Bootstrap

Register your service inside `src/bootstrap.rs` in `handle_services(...)`:

- Add routes to OpenAPI registration.
- Add background registration call(s).

## Step 5: Keep Boundaries Clear

- Controller: request parsing + response shaping.
- Service: business logic.
- Repository: DB reads/writes.
- Job handler: async side effects.

Avoid putting DB calls directly inside controllers.

## Step 6: Add Tests

At minimum:

- Unit tests for service logic.
- Integration tests for repository queries.
- E2E route tests for key flows.

## Step 7: Migrations

If schema changes are needed:

1. Add migration in `migration/src`.
2. Register it in `migration/src/lib.rs`.
3. Verify `AUTO_MIGRATE` behavior in local/dev environments.

## Step 8: Update Docs

Update:

- `docs/architecture.md` capability map.
- `docs/folder-structure.md` if structure changes.
- `README.md` if setup/feature list changes.

## Checklist

- [ ] Capability compiles and routes mount correctly.
- [ ] Private endpoints protected by auth middleware.
- [ ] Background jobs are idempotent and observable.
- [ ] Migrations included and reversible.
- [ ] Tests pass locally and in CI.
- [ ] Docs updated.
