# mbo-orderbook

Market-By-Order (MBO) data tools: DBN replay, streaming, consumption, and order book reconstruction.

Rust playground for MBO order book tools:

- `mbo-replay` – reads a DBN file, decodes MBO records, pretty-prints them.
- `mbo-streamer-raw` – streams a DBN file as raw bytes over TCP.
- `mbo-streamer` – decode+encode DBN streamer (buffered/streaming modes).
- `mbo-consumer` – connects to a streamer, decodes DBN, prints records.
