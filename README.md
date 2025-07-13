# Solana Caching Web Service

A simple, efficient Rust-based web service for caching recently confirmed slots on the Solana blockchain. This service
provides a fast way to check if a recent slot has been confirmed via a lightweight HTTP API.

The project is built with a focus on correctness, testability, and clean architecture, utilizing modern asynchronous
Rust.

-----

## Features

* **Continuous Caching**: A background service continuously polls for the latest confirmed slots and stores them in a
  fixed-size in-memory cache.
* **Configurable**: Cache capacity and RPC polling interval can be configured via a `.env` file.
* **HTTP API**: Exposes a single, simple endpoint (`/isSlotConfirmed/:slot`) to check the confirmation status of a given
  slot.
* **RPC Fallback**: If a requested slot is not found in the cache (i.e., it's an older slot), the service automatically
  falls back to a live RPC query to provide a definitive answer.
* **High Performance**: Built on `tokio` and uses a lock-free concurrent hash map for fast cache lookups.
* **Testable**: Includes a comprehensive test suite with mocked dependencies, allowing for complete validation without
  requiring a live network connection.
* **Observable**: Implements a metrics trait to record key events, such as RPC call durations and latest cached slots,
  which are logged to the console.
* **Dockerized**: Includes Dockerfile and docker-compose.yaml for a consistent, cross-platform development environment.
* **Live Reload**: The Docker setup is configured with cargo-watch for automatic recompilation and application restart
  on code changes.

-----

## Architecture Overview

The service is designed with a clean separation of concerns, organized into several distinct layers:

* **Handler Layer**: Manages incoming HTTP requests and outgoing responses. It's kept "lean" by delegating all business
  logic to the service layer.
* **Service Layer**: Contains the core application logic. This includes the background polling task and the confirmation
  checking logic.
* **Component Layer**: Consists of self-contained components like the RPC client and the in-memory cache.
* **Dependency Injection**: The application heavily uses traits (`RpcApi`, `Metrics`) and trait objects (`Arc<dyn ...>`)
  for dependency injection. This decouples the components and makes the entire application highly testable.

The entire application is containerized using Docker, ensuring a consistent environment for both development and
deployment.

-----

## Setup and Running

### Running with Docker (Recommended for Development)

This is the easiest way to get the service running with live-reloading enabled.

**Prerequisites**:

* [Docker](https://www.docker.com/products/docker-desktop/)

**1. Docker Files**

`Dockerfile` in the project root:

```dockerfile
FROM rust:1.86

RUN cargo install cargo-watch
WORKDIR /app
COPY . .
RUN cargo fetch

CMD ["cargo", "watch", "-x", "run"]
```

`docker-compose.yaml` file in the project root.

```yaml
services:
  solana-caching-service:
    container_name: solana-caching-service
    build:
      context: .
      dockerfile: Dockerfile
    restart: always
    ports:
      - "8000:8000"
    volumes:
      - ./src:/app/src
      - ./tests:/app/tests
      - ./Cargo.toml:/app/Cargo.toml
      - ./Cargo.lock:/app/Cargo.lock
      - ./.env:/app/.env
      - /app/target
```

**2. `.env` file**

`.env` file in the project root:

```env
# .env.example

# The base URL for the Solana RPC endpoint
SOLANA_RPC_URL=https://solana-mainnet.api.******.***/api-key/

# API key
API_KEY=*******

# Polling interval for the background service in seconds
POLL_INTERVAL_SECONDS=5

# The maximum number of slots to hold in the cache
CACHE_CAPACITY=10000
```

**3. Run the Service**

```sh
docker compose up --build
```

The service will be available at `http://localhost:8000`. Any changes saved to the `.rs` source files will trigger an
automatic restart of the service inside the container.

### Running Locally

**1. Configuration**

`.env` file in the root of the project directory. Example below:

```env
# .env.example

# The base URL for the Solana RPC endpoint
SOLANA_RPC_URL=https://solana-mainnet.api.******.***/api-key/

# API key
API_KEY=*******

# Polling interval for the background service in seconds
POLL_INTERVAL_SECONDS=5

# The maximum number of slots to hold in the cache
CACHE_CAPACITY=10000
```

**2. Build and Run**

To build and run the service:

```sh
cargo run
```

-----

## API Endpoint

### Check if a Slot is Confirmed

* **Endpoint**: `GET /isSlotConfirmed/:slot`
* **Description**: Checks if the given slot number corresponds to a confirmed block.
* **Example**:
  ```sh
  curl http://localhost:8000/isSlotConfirmed/234567890
  ```
* **Responses**:
    * **`200 OK`**: The slot is confirmed.
    * **`404 Not Found`**: The slot is not confirmed.
    * **`500 Internal Server Error`**: An unexpected error occurred (e.g., the RPC endpoint was unreachable).

-----

## Running Tests

The project includes a full test suite that mocks all external dependencies. To run the tests:

```sh
cargo test
```





























