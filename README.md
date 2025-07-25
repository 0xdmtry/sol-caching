# Multi-Tier Caching Service for Solana

A simple, efficient Rust-based web service for caching recently confirmed slots on the Solana blockchain. This service
provides a fast way to check if a recent slot has been confirmed via a lightweight HTTP API.

The project is built with a focus on correctness, testability, and clean architecture, utilizing modern asynchronous
Rust.

-----

## Features

* **Continuous Caching**: A background service continuously polls for the latest confirmed slots and stores them in a
  fixed-size in-memory cache.
* **Two-Tier Caching**: Implements a secondary LRU cache for older, on-demand slot lookups, significantly reducing
  redundant RPC calls for frequently queried historical data.
* **Fault-Tolerant Polling**: The background service includes a configurable retry mechanism with exponential backoff,
  making it resilient to transient RPC errors.
* **Configurable**: Cache capacity, polling interval, and retry strategy can be configured via a `.env` file.
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
* **Component Layer**: Consists of self-contained components like the RPC client and a two-tier in-memory cache system (
  a primary cache for recent slots and a secondary LRU cache for on-demand lookups).
* **Dependency Injection**: The application heavily uses traits (`RpcApi`, `Metrics`) and trait objects (`Arc<dyn ...>`)
  for dependency injection. This decouples the components and makes the entire application highly testable.

The entire application is containerized using Docker, ensuring a consistent environment for both development and
deployment.

-----

## Caching Strategy

The service uses a three-tiered lookup strategy to provide fast and efficient responses while minimizing the load on the
RPC provider. When a request for a slot is received, the service performs the following steps in order:

1. **Check Primary Cache (Tier 1)**: It first checks the primary `SlotCache`, which holds the most recent slots
   populated by the background poller. This is the fastest path for recently confirmed slots.

2. **Check LRU Cache (Tier 2)**: If the slot is not found in the primary cache, it then checks the secondary `LruCache`.
   This cache stores older slots that have been previously requested on-demand. This provides a fast path for "hot" but
   older slots.

3. **RPC Fallback (Tier 3)**: If the slot is not found in either cache, the service performs a fallback query to the
   Solana RPC endpoint.

4. **Populate LRU Cache**: If the RPC query successfully confirms the slot, the slot number is then added to the
   `LruCache` before the response is sent. This ensures that subsequent requests for the same older slot will be served
   quickly from the Tier 2 cache.

-----

## Fault Tolerance

The service is designed to be highly resilient to transient network or RPC provider issues through a multi-layered
defense strategy.

### Circuit Breaker Pattern

The service implements a circuit breaker to protect itself from repeatedly calling a downstream service that is known to
be failing.

* **Monitoring**: The circuit breaker tracks consecutive failures from RPC calls.
* **Opening the Circuit**: If the number of failures exceeds a configurable threshold, the circuit "opens." While open,
  all subsequent RPC calls are blocked immediately, returning an error instantly. This prevents the application from
  wasting resources on a failing dependency and gives the external service time to recover.
* **Recovery**: After a configured timeout, the circuit moves to a "half-open" state, allowing a single test request
  through. If it succeeds, the circuit closes and normal operation resumes.

### Intelligent Retry with Exponential Backoff

Wrapped inside the circuit breaker, the service uses a "smart" retry mechanism for individual RPC calls. It classifies
errors to decide whether an operation is worth retrying.

* **Transient Errors**: If a call fails with a temporary network issue (e.g., a timeout), the service will retry with an
  increasing delay (exponential backoff) up to a configurable maximum number of attempts.
* **Permanent Errors**: If a call fails with a non-transient error (e.g., an invalid API key), it will fail immediately
  without retrying.

### Configuration Guard Logic

To prevent a situation where retry delays could overlap with the next scheduled poll, the application validates the
configuration on startup. If the retry strategy is incompatible with the polling interval, the service will refuse to
start with a clear error message, ensuring predictable behavior.

### Graceful Shutdown

The service implements a graceful shutdown mechanism. When a shutdown signal (like `Ctrl+C`) is received, the web server
stops accepting new connections, allows in-flight requests to complete, and cleanly stops the background polling task
before the application terminates.

Example of output

![Shutdown](./assets/shutdown.png)

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

# The maximum number of slots to hold in the LRU cache
LRU_CACHE_CAPACITY=1000

# The maximum number of times to retry a failed RPC call
MAX_RETRIES=3

# The initial delay for the first retry, in milliseconds
INITIAL_BACKOFF_MS=500

# Number of consecutive failures before the circuit opens
CIRCUIT_FAILURE_THRESHOLD=5

# How long the circuit stays open before moving to half-open, in seconds
CIRCUIT_OPEN_DURATION_SECS=30
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

# The maximum number of slots to hold in the LRU cache
LRU_CACHE_CAPACITY=1000

# The maximum number of times to retry a failed RPC call
MAX_RETRIES=3

# The initial delay for the first retry, in milliseconds
INITIAL_BACKOFF_MS=500

# Number of consecutive failures before the circuit opens
CIRCUIT_FAILURE_THRESHOLD=5

# How long the circuit stays open before moving to half-open, in seconds
CIRCUIT_OPEN_DURATION_SECS=30
```

**2. Build and Run**

To build and run the service:

```sh
cargo run
```

-----

## API Endpoint

The project includes a Postman collection, i.e. solana-caching-service.postman_collection.json

### Health Check

A simple health check endpoint that returns a `pong` response, which can be used to verify that the service is running.

* **Endpoint**: `GET /`
* **Example**:
  ```sh
  curl http://localhost:8000/
  ```
* **Response**:
    * **`200 OK`**: with the plain text body `pong`.

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

### Cache Inspection Endpoints

These endpoints provide a view into the current state of the caches for debugging and observability.

#### Get Latest Slots

Returns all slots currently held in the primary cache (the most recent slots).

* **Endpoint**: `GET /cache/latest`
* **Example**:
  ```sh
  curl http://localhost:8000/cache/latest
  ```
* **Response**:
    * **`200 OK`**: with a JSON body containing an array of slot numbers, e.g., `[353110300, 353110301, 353110302]`.

#### Get LRU Slots

Returns all slots currently held in the secondary LRU cache (older, on-demand slots).

* **Endpoint**: `GET /cache/lru`
* **Example**:
  ```sh
  curl http://localhost:8000/cache/lru
  ```
* **Response**:
    * **`200 OK`**: with a JSON body containing an array of slot numbers, e.g., `[234567890, 198765432]`.

-----

## Running Tests

The project includes a full test suite that mocks all external dependencies. To run the tests:

```sh
cargo test
```

Example of output:

![Tests](./assets/tests.png)