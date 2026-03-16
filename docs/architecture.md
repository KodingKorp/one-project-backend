# Architecture

## Runtime Overview

The server starts in `src/main.rs`, loads config, initializes logging, builds the app through `bootstrap::build_app()`, then binds to `HOST:PORT`.

`build_app()` composes:

- Shared app state (`AppState`) with a SeaORM DB connection.
- Redis-backed server session middleware.
- Background orchestrator startup and registration.
- Public and private OpenAPI route trees.
- Middleware stack (cookies, sessions, CORS, tracing, panic handling).

## API Surfaces

- Public APIs: `/api/v1/public`
- Private APIs (authorization middleware): `/api/v1`
- Public Swagger UI: `/swagger`
- Private Swagger UI: `/private/swagger`
- Static assets: `/`

## Core Capability Modules

### IAM (`src/capabilities/iam`)

- Auth flows: register/login with password and magic-link.
- Session object persistence in Redis session storage.
- Organisation and user-to-organisation mapping (role + status).
- Role-aware organisation operations (invite, switch org, deactivate org, etc.).
- IAM-related background handlers for registration side effects and scheduled jobs.

### Notifications (`src/capabilities/notifications`)

- Email message object and handler.
- Background queue registration (`notification`) and email job dispatch.
- SMTP health probing through `Mailer`.

### Background (`src/capabilities/background`)

- Queue manager and orchestrator for queue/job/schedule registration.
- Worker execution with job status transitions.
- Persistent job records in Postgres (`jobs` table).
- Poll-based scheduler for cron-triggered jobs.

### Shared Platform

- Config loading: `src/capabilities/config.rs`
- Logging: `src/capabilities/logger.rs`
- DB/Redis connections + optional migrations: `src/capabilities/database.rs`
- Mail transport + template rendering: `src/capabilities/mailer.rs`
- Crypto/JWT/password hashing: `src/capabilities/crypto.rs`

## Service Registration Pattern

Every capability should follow the `Service` trait contract (`src/capabilities/lib/service_trait.rs`):

- `register_routes()`
- `register_private_routes()`
- `register_background()`

This is the extension seam used by `bootstrap::handle_services(...)`.

## Persistence Model

SeaORM migrations in `migration/src/*` define:

- `users`
- `sessions`
- `user_login`
- `jobs`
- `organisations`
- `organisation_to_user_mapping`

## Local Dependencies

`dev_infra/docker-compose.yml` provides local Postgres, Redis, and SMTP4Dev for development/testing.
