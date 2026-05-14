# kv6

A Redis-like KV database with LSM-Tree persistent storage written in Rust.

## Build & Test

```bash
cargo build --release
```

Run the full verification suite (fmt, clippy, test):
```bash
./test.sh
```

Run a single test:
```bash
cargo test <test_name>
```

Run the server:
```bash
cargo run --bin kv6-server
```

Run the CLI:
```bash
cargo run --bin kv6-cli
```

## Binaries

- `kv6-server` - Entry point: `src/main.rs`
- `kv6-cli` - Entry point: `src/cli.rs`

## Verification Order

`cargo fmt --check` -> `cargo clippy -- -D warnings` -> `cargo test`

This project uses clippy with `-D warnings` (treats warnings as errors).

## Architecture

- `src/cmd.rs` - Command parsing and handling
- `src/server.rs` - Server setup
- `src/db.rs` - Core key-value store
- `src/pubsub.rs` - Pub/sub functionality
- `src/resp.rs` - RESP protocol encoding/decoding
- `src/store/` - Storage implementations (LSM-Tree)
- `src/lsm/` - LSM-Tree core implementation (v6.1+)

## LSM-Tree Architecture (v6.1+)

```
Write Path:
  PUT -> MemTable (SkipList) -> Flush -> SSTable (L0) -> Compaction -> SSTable (L1+)

Read Path:
  MemTable -> Bloom Filter -> SSTable (L0) -> SSTable (L1) -> ...
```

- `src/lsm/memtable.rs` - In-memory SkipList
- `src/lsm/sstable.rs` - SSTable file format
- `src/lsm/compaction.rs` - Background compaction
- `src/lsm/builder.rs` - SSTable builder