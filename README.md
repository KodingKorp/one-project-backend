# One Project Backend

A Rust backend REST API foundation from KodingKorp for building robust products with shared platform capabilities.

## Features

- IAM: password and magic-link auth, sessions, organisation membership and role mapping.
- Swagger/OpenAPI: auto-generated public and private API docs.
- Background jobs: queue + worker + scheduler model.
- Notifications: email delivery through background handlers.
- Platform utilities: logging, config loading, database and Redis integrations.

## Getting Started

### Prerequisites

- Rust (stable)
- Docker

### Setup

1. Clone repository:

```sh
git clone https://github.com/KodingKorp/one-project-backend.git
cd one-project-backend
```

2. Copy env files:

```sh
cp .env.example .env
cp .env.example .env.e2e.test
```

3. Start local infra (Postgres, Redis, SMTP):

```sh
docker compose -f dev_infra/docker-compose.yml up -d
```

4. Run migrations:

```sh
cargo install sea-orm-cli@1.1.0
sea-orm-cli migrate up
```

5. Run server:

```sh
cargo run --release
```

## API Docs

- Public Swagger UI: `/swagger`
- Private Swagger UI: `/private/swagger`

## Documentation

- Project docs index: [`docs/README.md`](docs/README.md)
- Agent/contributor guardrails: [`AGENTS.md`](AGENTS.md)

## Project Structure

- `src/`: application source
- `migration/`: database migrations
- `dev_infra/`: local infra definitions
- `templates/`: email templates

## Contributing

Contributions are welcome via issues and pull requests.

## License

MIT. See [`LICENSE`](LICENSE).
