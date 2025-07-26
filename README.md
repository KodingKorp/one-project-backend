# One Project Backend

A Rust backend REST API server template to quickly build out robust projects.

## Features

- **Auth**: Password-based and Magic Link authentication
- **Swagger**: Auto-generated OpenAPI documentation
- **Cron Jobs**: Schedule and manage recurring tasks
- **Background Jobs**: Asynchronous job processing
- **Email Notifications**: Built-in email notification support

## Getting Started

### Prerequisites
- Rust (latest stable)
- Docker (for running dependencies like Postgres, Redis, SMTP)

### Setup

1. **Clone the repository:**
   ```sh
   git clone https://github.com/KodingKorp/one-project-backend.git
   cd one-project-backend
   ```

2. **Copy environment files:**
   ```sh
   cp .env.example .env
   cp .env.example .env.e2e.test
   ```

3. **Start dependencies (Postgres, Redis, SMTP):**
   ```sh
   docker compose -f dev_infra/docker-compose.yml up -d
   ```

4. **Run database migrations:**
   ```sh
   cargo install sea-orm-cli@1.1.0
   sea-orm-cli migrate up
   ```

5. **Build and run the server:**
   ```sh
   cargo build --release
   cargo run --release
   ```

## API Documentation

- Swagger UI is available at `/docs` when the server is running.

## Project Structure

- `src/` - Main application code
- `migration/` - Database migrations
- `dev_infra/` - Development infrastructure (docker-compose, SQL setup)
- `templates/` - Email and notification templates

## Contributing

Contributions are welcome! Please open issues or submit pull requests.

## License

MIT License. See [LICENSE](LICENSE) for details.
