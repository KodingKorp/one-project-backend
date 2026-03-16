# Folder Structure

The folder structure is intentionally semantic. Keep this contract stable.

## Top Level

- `src/`: application source code.
- `migration/`: database schema migrations.
- `dev_infra/`: local dependency infra (Postgres, Redis, SMTP test server).
- `templates/`: email templates.
- `static/`: static files served by the app.
- `compose.yml`: app container definition.
- `Dockerfile`: multi-stage build for deployable backend image.

## `src/`

- `main.rs`: process entry point.
- `bootstrap.rs`: dependency and route composition.
- `base/`: base service endpoints (ping/health).
- `capabilities/`: capability modules.
- `test_utils.rs`: shared helpers for tests.

## `src/capabilities/`

- `iam/`: auth, sessions, organisations, role mapping, IAM background handlers.
- `notifications/`: email notification dispatch via background queue.
- `background/`: queue/orchestrator/worker scheduler engine.
- `lib/`: shared service trait, common response, common error.
- `config.rs`, `database.rs`, `logger.rs`, `mailer.rs`, `crypto.rs`: shared platform utilities.

## Capability Internal Layout

Use this layout for each capability:

- `controllers/`: HTTP route handlers only.
- `services/`: business logic and orchestration.
- `repositories/`: persistence and query layer.
- `entities/`: SeaORM entities.
- `job_handlers/`: background job execution units.
- `objects.rs`: API-facing DTOs.
- `service.rs`: capability registration via `Service` trait.

## Why This Matters

- The structure communicates boundaries at a glance.
- New contributors can navigate quickly.
- Product capability additions remain consistent and low-risk.
