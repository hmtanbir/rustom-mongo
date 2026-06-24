# Rustom

A production-ready, highly scalable, and concurrent Rust API boilerplate template.

Rustom is built with **Axum**, leveraging Rust's fearless concurrency and performance to deliver a robust backend architecture. It incorporates best practices such as Domain-Driven Design (DDD), robust authentication, caching, message queues, and auto-generated API documentation.

## 🚀 Features

- **Blazing Fast Web Framework:** Built on top of [Axum](https://github.com/tokio-rs/axum) and [Tokio](https://tokio.rs/) for maximum performance and ergonomics.
- **Secure Authentication:** JWT-based authentication with secure password hashing using Argon2id.
- **Role-Based Access Control (RBAC):** Built-in middleware for route protection and role-based permissions (e.g., Admin vs User). Roles are dynamically mapped from the `roles.yml` configuration file.
- **PostgreSQL Database:** Async database interaction and compile-time SQL query verification using [SQLx](https://github.com/launchbadge/sqlx). Automatic database migrations on startup.
- **Redis Caching:** High-performance data caching layer using Redis to minimize database load.
- **RabbitMQ Background Workers:** Asynchronous background job processing and message queuing.
- **Interactive API Documentation:** Auto-generated Swagger UI and OpenAPI specifications via [utoipa](https://github.com/juhaku/utoipa).
- **Containerized Development:** Complete `docker-compose` setup for the API, PostgreSQL, Redis, and RabbitMQ. Includes a multi-stage, highly optimized Dockerfile for production.
- **Testing Strategy:** Built-in unit tests and mocked services using `mockall`.

## 📂 Project Structure

The project follows a modular, clean architecture approach:

```text
src/
├── api/            # API Layer: Axum routers, handlers, and middlewares (Extractors, Auth)
├── config/         # Configuration logic (Environment variables via dotenvy)
├── docs/           # OpenAPI documentation configurations
├── domain/         # Domain Layer: Core models, DTOs, Enums, and AppErrors
├── infrastructure/ # Infrastructure Layer: Database (Postgres), Cache (Redis), and MQ (RabbitMQ) connections
└── services/       # Service Layer: Core business logic, integrating with infrastructure
```

## 🛠️ Prerequisites

To run the project, you will need:
- [Docker](https://docs.docker.com/get-docker/) & Docker Compose
- [Rust](https://www.rust-lang.org/tools/install) (If running natively without Docker)

## 🐳 Running with Docker (Recommended)

The easiest way to run the application locally is using Docker Compose. This spins up the API, PostgreSQL, Redis, and RabbitMQ containers simultaneously.

1. **Copy the example environment file:**
   ```bash
   cp .env.example .env
   ```
   *(Update the `.env` values if necessary)*

2. **Build and start the containers:**
   ```bash
   docker compose up --build
   ```

The API will be available at `http://localhost:3000`.

## 💻 Running Natively (Without Docker)

If you prefer to run the Rust application natively on your machine, you must have Postgres, Redis, and RabbitMQ running locally or remotely.

1. **Start the infrastructure services (using Docker):**
   ```bash
   docker compose up postgres redis rabbitmq -d
   ```

2. **Configure your `.env` and `.env.test` and `.env.production` files:**
   Make sure the database, Redis, and RabbitMQ connection variables point to your running instances.

3. **Create the Database & Run Migrations:**
   You must create the database before running the application natively. You can use `sqlx-cli` to handle this.
   
   First, install `sqlx-cli` (specifying a version if your rustc is outdated):
   ```bash
   cargo install sqlx-cli --version="0.8.2"
   ```

   Then, create the database and apply the schema migrations:
   ```bash
   sqlx database create
   sqlx migrate run --source db/migrations
   ```



   #### 🌐 Target Other Environments (Test, Production, etc.)
   Since `sqlx-cli` reads from `.env` by default and does not natively parse `APP_ENV` to switch files, you need to explicitly supply the database URL for other environments. You can do this by setting the `DATABASE_URL` inline or using the `-D` flag:

   **For Test Environment:**
   ```bash
   # Option 1: Inline environment variable
   sqlx database create
   sqlx migrate run --source db/migrations
   
   # Option 2: Using the DATABASE_URL flag (-D)
   sqlx database create -D postgres://postgres:postgres@localhost:5432/rustom_test
   sqlx migrate run --source db/migrations -D postgres://postgres:postgres@localhost:5432/rustom_test
   ```

   **For Production Environment:**
   ```bash
   sqlx database create
   sqlx migrate run --source db/migrations
   ```

   
   If you need to **rollback/revert** the latest migration, you can use:
   ```bash
   sqlx migrate revert --source db/migrations
   ```

   To **seed data** into the database using SQLx, you can run the SQL files in the `db/seeds/` directory as a custom migration source (using `--ignore-missing` so SQLx ignores standard migrations from the default folder):
   ```bash
   sqlx migrate run --source db/seeds --ignore-missing
   ```

   To **re-run seeds** after they have been modified (e.g. to update mock data), you have two options:
   
   *Option A: Reset the database completely (standard for local development)*
   ```bash
   sqlx database reset -y --source db/migrations
   sqlx migrate run --source db/seeds --ignore-missing
   ```
   
   *Option B: Clear the seed migration history entry and re-run (without resetting)*
   1. Connect to your database using your preferred SQL client and execute:
      ```sql
      DELETE FROM _sqlx_migrations WHERE version = 20260612000001;
      ```
   2. Re-run the seed command:
      ```bash
      sqlx migrate run --source db/seeds --ignore-missing
      ```

4. **Run the application:**
   ```bash
   cargo run
   ```


## 🌍 Environments

The application supports different environments (similar to Rails) via the `APP_ENV` environment variable. 

The configuration loading behaves as follows:
- **`APP_ENV`** can be set to `development`, `test`, or `production`. If not set, it defaults to `development`.
- It loads variables from `.env.[APP_ENV]` (e.g. `.env.development`, `.env.test`, `.env.production`).
- If the environment-specific file is missing, it falls back to the default `.env` file.

### How to Run:
```bash
# Run in development (default)
cargo run

# Run in production
APP_ENV=production cargo run

# Run in test environment
APP_ENV=test cargo run
```

## 🗑️ Clearing the Cache

The application uses Redis to cache API responses (such as the paginated users index). If you need to instantly wipe the cache and force the API to fetch fresh data from the database, you can run:

```bash
# If running Redis via Docker Compose
docker compose exec redis redis-cli flushall

# If running Redis natively
redis-cli flushall
```

## 📚 API Documentation

Once the server is running, you can explore and test the API endpoints using the auto-generated Swagger UI interface.

Navigate to:
👉 **[http://localhost:3000/api-docs](http://localhost:3000/api-docs)**

## 🧪 Running Tests

To execute the unit and integration tests:

```bash
cargo test
```

## 🧹 Linting and Formatting

To check for and automatically apply Clippy fixes:

```bash
cargo clippy --fix --lib -p rustom --allow-dirty
```

## 📜 License

This project is open-source and available under the [MIT License](LICENSE).

