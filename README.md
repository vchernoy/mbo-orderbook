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
