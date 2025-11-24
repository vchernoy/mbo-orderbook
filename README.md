# mbo-orderbook

Market-By-Order (MBO) data tools: DBN replay, streaming, consumption, and order book reconstruction.

Rust playground for MBO order book tools:

- `mbo-replay` – reads a DBN file, decodes MBO records, pretty-prints them.
- `mbo-streamer-raw` – streams a DBN file as raw bytes over TCP.
- `mbo-streamer` – decode+encode DBN streamer (buffered/streaming modes).
- `mbo-consumer` – connects to a streamer, decodes DBN, prints records.

`mbo-orderbook` is a Rust toolkit for working with **Market-By-Order (MBO)** market data in **Databento DBN format**.
It includes tools for DBN replay, TCP streaming, raw feeding, and real-time MBO consumption.
The project is structured as a multi-binary Rust workspace, designed to resemble a production-style market data pipeline.

---

## ✨ Features

### 🔹 Raw DBN Streaming (Zero-Copy)

- Streams DBN files directly over TCP without decoding.
- Supports:
  - **buffered mode** – file loaded once into memory,
  - **streaming mode** – chunked reads per client (constant memory).
- Ideal for throughput tests and multi-client distribution.

### 🔹 Decode + Encode DBN Streamer

- Decodes DBN metadata & records with `AsyncDbnDecoder`.
- Re-encodes records using `AsyncDbnEncoder`.
- Allows filtering, transformation, rate limiting, etc.
- Two modes:
  - **buffered** (load once),
  - **streaming** (decode per-connection).

### 🔹 MBO Consumer

- Connects to any DBN TCP stream.
- Parses metadata and `MboMsg` messages.
- Pretty-prints:
  - action (A/M/C/F/T)
  - side (B/A)
  - fixed-precision DBN prices (1e-9)
  - quantity, order ID, timestamps
  - instrument IDs

### 🔹 DBN Replay (local)

- Reads DBN files.
- Prints parsed `MboMsg` records (debug or pretty).
- Useful for debugging and inspecting raw data.

### 🔹 Modular Architecture

Each tool is implemented as an independent CLI binary:

src/bin/mbo-consumer.rs
src/bin/mbo-streamer.rs
src/bin/mbo-streamer-raw.rs
src/bin/mbo-replay.rs

This layout makes the project easy to extend (e.g., orderbook engine, HTTP API, WebSocket API, backtester, etc.).

---

## 🚀 Build

Compile all binaries:

```bash
cargo build --release
```

Executables will appear under:

target/release/

### 📦 Usage Examples

#### 1️⃣ Replay DBN locally

Print parsed MboMsg records:

```bash
cargo run --bin mbo-replay -- CLX5_mbo.dbn
```

---

#### 2️⃣ Raw DBN Streamer (Zero-Copy)

Buffered mode (default):

File is loaded once and streamed to all clients.

```bash
cargo run --bin mbo-streamer-raw -- CLX5_mbo.dbn --bind 127.0.0.1:5000
```

Streaming mode (reads per client):

Constant memory footprint.

```bash
cargo run --bin mbo-streamer-raw -- CLX5_mbo.dbn \
    --bind 127.0.0.1:5000 \
    --mode streaming
```

#### 3️⃣ Decode+Encode DBN Streamer

Buffered mode:

```bash
cargo run --bin mbo-streamer -- CLX5_mbo.dbn \
    --bind 127.0.0.1:5000 \
    --mode buffered
```

Streaming mode:

```bash
cargo run --bin mbo-streamer -- CLX5_mbo.dbn \
    --bind 127.0.0.1:5000 \
    --mode streaming
```

---

#### 4️⃣ MBO Consumer

Connect to any DBN-over-TCP stream:

```bash
cargo run --bin mbo-consumer
```

Custom address:

```bash
cargo run --bin mbo-consumer -- --addr 127.0.0.1:5000
```

Pretty-print mode:

```bash
cargo run --bin mbo-consumer -- --pretty
```

Limit output:

```bash
cargo run --bin mbo-consumer -- --limit 100
```

---

🔁 End-to-End Example

Start streamer:

```bash
cargo run --bin mbo-streamer -- CLX5_mbo.dbn --bind 127.0.0.1:5000
```

Start consumer:

```bash
cargo run --bin mbo-consumer -- --addr 127.0.0.1:5000 --pretty
```

You will see MBO events flowing in real time.

---

---

## 🐳 Docker Support

This project provides a multi-stage Dockerfile for building Rust binaries in a builder image and packaging them into a minimal, production-ready runtime image.

This allows you to run the MBO streamers and consumers without installing Rust locally.

### 📦 Build the Docker Image

From the repository root:

```bash
docker build -t mbo-orderbook .
```

This will:

compile all Rust binaries in release mode

produce a clean runtime image containing:

mbo-replay
mbo-streamer
mbo-streamer-raw
mbo-consumer

### 🚀 Run Examples

#### 1️⃣ Stream DBN data using the decode+encode streamer

```bash
docker run --rm -p 5000:5000 \
  -v "$PWD:/data" \
  mbo-orderbook \
  mbo-streamer /data/CLX5_mbo.dbn --bind 0.0.0.0:5000 --mode buffered
```

Then on the host:

```bash
cargo run --bin mbo-consumer -- --addr 127.0.0.1:5000 --pretty
```

#### 2️⃣ Run the zero-copy raw streamer

```bash
docker run --rm -p 5000:5000 \
  -v "$PWD:/data" \
  mbo-orderbook \
  mbo-streamer-raw /data/CLX5_mbo.dbn --bind 0.0.0.0:5000 --mode streaming
```

#### 3️⃣ Replay DBN data inside Docker

```bash
docker run --rm \
  -v "$PWD:/data" \
  mbo-orderbook \
  mbo-replay /data/CLX5_mbo.dbn
```

#### 4️⃣ Run the consumer from inside Docker

```bash
docker run --rm \
  -v "$PWD:/data" \
  mbo-orderbook \
  mbo-consumer --addr 127.0.0.1:5000 --pretty
```

(You can run the consumer inside Docker or locally.)

#### 📁 Mounted Volumes

The examples mount the current directory:

```bash
-v "$PWD:/data"
```

This lets the container access your .dbn files.

### 🛠 Dockerfile Overview

The included Dockerfile uses two stages:

1. builder (rust:1.91-bookworm)

   - builds binaries in release mode

2. runtime (debian:bookworm-slim)
   - copies only the compiled binaries
   - extremely small final image
   - includes a non-root user

This makes builds fast and the runtime image secure and lightweight.
