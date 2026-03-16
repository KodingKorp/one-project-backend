# AGENTS.md

This repository is an open-source backend foundation for product teams.

The main intent is: ship enterprise-ready defaults once, then let teams add new business capabilities with minimal friction.

## Project Goal

`one-project-backend` provides default, reusable platform capabilities:

- Identity and access (auth, sessions, organisation mapping, role-based checks)
- Notifications and email
- Background jobs and schedules
- Logging and config loading
- Database and migrations
- Health endpoints and API docs
- Test scaffolding

Product teams should primarily add a new capability module, not re-architect shared foundations.

## Architecture Snapshot

- `src/main.rs`: process bootstrap and server startup.
- `src/bootstrap.rs`: app wiring, middleware stack, route registration, background orchestrator startup.
- `src/capabilities/*`: feature and platform modules.
- `src/base/*`: root endpoints (`/ping`, `/health`) and base service.
- `migration/*`: SeaORM migrations.
- `dev_infra/*`: local dependency stack (Postgres, Redis, SMTP test server).

## Capability Contract

When creating a new product capability, follow this contract:

1. Create a module in `src/capabilities/<capability_name>/`.
2. Expose it in `src/capabilities/mod.rs`.
3. Add a `<Capability>Service` implementing `Service` (`src/capabilities/lib/service_trait.rs`).
4. Register public/private OpenAPI routes through that service.
5. If needed, register queue(s), job handler(s), and schedule(s) in `register_background`.
6. Keep data access inside `repositories`, request/response shapes in `objects`, and DB models in `entities`.

## Folder Semantics

- `controllers`: HTTP/OpenAPI handlers only.
- `services`: orchestration/business logic.
- `repositories`: database queries and persistence.
- `entities`: SeaORM entities.
- `job_handlers`: background job execution logic.
- `objects`: API-facing DTOs.

Do not place cross-cutting business logic directly in controllers.

## Definition of Done For New Capabilities

- Routes registered and visible in Swagger.
- Auth boundary clear (public vs private).
- Required background jobs registered and health-safe.
- Migrations included for any schema changes.
- Unit/integration/e2e tests added or updated.
- Docs updated under `docs/`.

## Operational Notes

- App expects Postgres + Redis + SMTP.
- Session middleware uses Redis storage.
- Private APIs are mounted under `/api/v1`; public APIs under `/api/v1/public`.
- Swagger endpoints:
  - Public: `/swagger`
  - Private: `/private/swagger`

## Collaboration Notes

- Preserve folder semantics; they are part of project design.
- Prefer small, composable services and explicit boundaries.
- Avoid introducing vendor lock-in in product capability code; keep integration adapters swappable.
