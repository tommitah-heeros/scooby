# Scooby

Scooby is a small Rust CLI for creating HTTP requests to services and exploring a history of previous requests stored in a database (Turso).

> Note: This is the authors "first" project in Rust, it sucks.

---

## Installation

From source:

```bash
git clone https://github.com/tommitah-heeros/scooby.git
cd scooby

# Build release binary
cargo build --release

# Optionally, put it on your PATH
cp target/release/scooby /usr/local/bin/
```

Requirements:

- Rust toolchain (via `rustup`)
- Turso CLI (if you want to store/query requests using Turso, not required for the program itself)

---

## Configuration
Add a `auth_token` variable to your shell that populates a Cookie header `export auth_token = "....."`.

Make sure to configure your service names in `~/.config/scooby/config.toml` like the following:
```toml
my_service = "someurlpart"
```

## Quick Start

Scooby is organized around **subcommands**:

- `req` – create and send HTTP requests to services
- `db` – inspect previously sent requests (list, filter, UI)

Basic pattern:

```bash
scooby <mode> [subcommand] [flags] [args]
```

Run help at any time:

```bash
scooby --help
scooby req --help
scooby db --help
```

---

## Request Mode: `req`

`req` is used to build and execute HTTP requests.

Signature (from `cli.rs`):

```text
scooby req <METHOD> <SERVICE> <ROUTE_URL> [FLAGS]
```

### Arguments

- `METHOD` (required)

  HTTP method used for the request.

  Backed by `reqwest::Method`, common values include:

  - `GET`
  - `POST`
  - `PUT`
  - `PATCH`
  - `DELETE`

  Example:

  ```bash
  scooby req GET ...
  scooby req POST ...
  ```

- `SERVICE` (required)

  Target service identifier. Intended usage:

  - Use a short **abbreviation** that maps to a full base URL in a future `config.toml`. `config.toml` should be located under `~/.config/scooby`.
  - Example user mapping:

    ```toml
    # config.toml
    user-svc = "https://users.example.com"
    order-svc = "https://orders.example.com"
    ```

  Then you could use:

  ```bash
  scooby req GET user-svc users/123
  ```

- `ROUTE_URL` (required)

  Resource route or path part (e.g. `users/123`, `orders?status=open`).

  Combined with `SERVICE`, environment and stack prefix to form the final URL.

### Flags

- `-b, --base <DOMAIN_URL>`

  Domain base url.

  - By default `scooby` looks for a value in `config.toml`: `domain_url = "https://my.domain.com"`
  - If domain url depends on `ENV`, use (exactly) the following in your `config.toml`: `domain_url = "https://my.[SERVER_ENV].domain.com"`. Placement of tag doesn't matter, but naming is enforced.

- `-d, --dev <DEV_PREFIX>`

  Dev-stack prefix for dev/test environments.

  - Default is an empty string `""`, but you can set this yourself.
  - Used to build stack names when targeting non-production environments.

  Example:

  ```bash
  # With an explicit dev prefix
  scooby req GET user-svc users/123 -d myprefix-
  ```

- `-s, --server <ENV>`

  Server environment. Backed by the `ServerEnv` enum:

  - `dev`
  - `test`
  - `prod` (mapped internally to `"cloud"`)

  If **omitted**, it defaults to `dev`.

  Examples:

  ```bash
  # Default dev
  scooby req GET user-svc users/123

  # Explicit test
  scooby req GET user-svc users/123 -s test

  # Production (cloud)
  scooby req GET user-svc users/123 -s prod
  ```

- `-q, --qsp <QUERYSTRING>`

  Optional raw query-string parameters.

  Examples:

  ```bash
  # Simple querystring
  scooby req GET user-svc users -q "status=active"

  # Multiple params
  scooby req GET order-svc orders -q "status=open&limit=50"
  ```

- `-p, --payload <PATH>`

  JSON payload file path for `POST` or `PATCH` requests.

  - Only required when:
    - `method == POST`, or
    - `method == PATCH`
  - For other methods, it is optional and typically unused.

  Examples:

  ```bash
  # POST with a JSON body
  scooby req POST user-svc users -p payloads/create-user.json

  # PATCH with a JSON body
  scooby req PATCH user-svc users/123 -p payloads/update-user.json
  ```

  The payload file should contain valid JSON.

---

## Database Mode: `db`

`db` is used to inspect or explore requests that Scooby has stored (in Turso).

Top-level pattern:

```bash
scooby db <SUBCOMMAND> [ARGS]
```

Available subcommands (from `DbCommand` in `cli.rs`):

- `list-all`
- `list-by-service`
- `ui`

Run:

```bash
scooby db --help
scooby db list-all --help
scooby db list-by-service --help
scooby db ui --help
```

### `db list-all`

List all stored requests in a given time range.

Signature (from `ListAllCommand`):

```text
scooby db list-all <TIME_RANGE>
```

- `TIME_RANGE` is a string. So far only supports an "up-to-date" but more extended usage is todo.

Examples:

```bash
# All requests before (including) Jan 1st 2026
scooby db list-all 2026-01-01
```

### `db list-by-service`

List requests filtered by a specific service and time range.

Signature (from `ListByServiceCommand`):

```text
scooby db list-by-service <SERVICE> <TIME_RANGE>
```

Arguments:

- `SERVICE` – the same service identifier used in `scooby req`
- `TIME_RANGE` – see above

Examples:

```bash
# Requests to user-svc before (including) 2026-01-01
scooby db list-by-service user-svc 2026-01-01
```

### `db ui`

Launches an interactive TUI to explore the stored requests.

Signature (from `UiCommand`):

```text
scooby db ui
```

Description:

- Implemented using [ratatui](https://github.com/ratatui-org/ratatui).
- Shows a list of previous requests on the left.
- Displays the payload and response for the selected request on the right.
- Supports a fullscreen mode for focusing on payload/response details.

Key bindings:

- `j` / `k` – Move selection up/down in the requests list (when not in fullscreen).
- `Enter` – Toggle fullscreen mode (split payload/response view).
- `Tab` – Switch focus between payload and response in fullscreen.
- `Ctrl+u` / `Ctrl+d` – Scroll the focused pane up/down.
- `q` – Quit the UI.

Usage:

```bash
scooby db ui
```

---

## Running Turso Locally (if you want to query db contents yourself, requires turso cli)

1. Start a dev Turso instance:

   ```bash
   turso dev --db-file <path>
   ```

   - Replace `<path>` with a path to the SQLite file you want Turso to use.

2. Turso will print the port it is listening on, then connect using:

   ```bash
   turso db shell <port>
   ```

3. Inside the Turso shell you can inspect tables, run queries, etc.

   ```sql
   .tables
   SELECT * FROM requests LIMIT 10;
   ```

How Scooby uses Turso (conceptually):

- Each outgoing request via `scooby req` is recorded in Turso:
  - timestamp
  - service
  - method
  - URL
  - status code
  - payload/response snippets (depending on implementation)
- The `db` subcommands read from this store.

---

## Examples

### Simple GET request

```bash
# Get user 123 from dev user-svc
scooby req GET user-svc users/123
```

### GET with querystring

```bash
# List active users with a limit
scooby req GET user-svc users -q "status=active&limit=20"
```

### POST with payload

```bash
# JSON payload in payloads/new-user.json
scooby req POST user-svc users -p payloads/new-user.json
```

### Inspect recent activity

```bash
# All requests until (including) Dec 27th 2025
scooby db list-all 2025-27-12

# Requests to user-svc until (including) Dec 27 2025
scooby db list-by-service user-svc 2025-27-12

# Browse everything in a TUI
scooby db ui
```
