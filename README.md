# waterfalls-client

[Waterfalls](https://github.com/RCasatta/waterfalls) API client library. Supports plaintext, TLS and Onion servers. Blocking or async.

## Overview

This library provides both blocking and asynchronous HTTP clients for interacting with Waterfalls servers. It supports querying transaction histories, UTXO data, and blockchain information using Bitcoin descriptors or addresses.

## Features

- **Blocking and Async clients** - Choose the right client for your use case
- **Waterfalls API support** - Query with descriptors or addresses  
- **Compatible endpoints** - Transaction retrieval, block headers, broadcasting
- **Proxy support** - SOCKS proxy support for privacy
- **TLS/SSL support** - Secure connections with multiple TLS backends
- **Retry logic** - Automatic retries for temporary failures

## Usage

### Creating a Client

```rust
use waterfalls_client::Builder;

// Blocking client
let builder = Builder::new("https://waterfalls.example.com/api");
let blocking_client = builder.build_blocking();

// Async client  
let builder = Builder::new("https://waterfalls.example.com/api");
let async_client = builder.build_async()?;
```

### Querying with Descriptors

```rust
// Query with a Bitcoin descriptor
let descriptor = "wpkh(xpub.../*)";
let response = client.waterfalls(descriptor).await?;

// Query with specific parameters
let response = client.waterfalls_version(descriptor, 2, None, None, false).await?;
```

### Querying with Addresses

```rust
use bitcoin::Address;

let addresses = vec![address1, address2];
let response = client.waterfalls_addresses(&addresses).await?;
```

### Compatible Endpoints

```rust
// Get transaction
let tx = client.get_tx(&txid).await?;

// Get block header  
let header = client.get_header_by_hash(&block_hash).await?;

// Get tip hash
let tip = client.get_tip_hash().await?;

// Broadcast transaction
client.broadcast(&transaction).await?;
```

## Testing

The library includes comprehensive unit and integration tests.

### Unit Tests

Run the unit tests (no external dependencies required):

```bash
nix develop -c cargo test --lib
```

### Simple Integration Tests

Basic integration tests that don't require external dependencies:

```bash
nix develop -c cargo test --test simple_integration
```

These verify client construction, error handling, and API signatures.

### Full Integration Tests

**Note**: Uses waterfalls 0.9.4+ with type conversions between waterfalls::be and bitcoin types.

Integration tests require a running Waterfalls server with bitcoind. The tests use the `waterfalls` crate's `test_env` feature to automatically set up test environments.

**Prerequisites:**
- Environment variables `BITCOIND_EXEC` and `ELEMENTSD_EXEC` are automatically set in the nix environment

**Run integration tests:**
```bash
nix develop -c cargo test --test integration
```

The integration tests cover:
- All waterfalls-specific endpoints (`waterfalls`, `waterfalls_addresses`, etc.)
- Compatible Esplora endpoints (`get_tx`, `get_header_by_hash`, etc.)  
- Server information endpoints (`server_recipient`, `server_address`)
- Both blocking and async clients
- Type conversions from waterfalls::be types to bitcoin types

## Development

This project uses Nix for reproducible development environments:

```bash
nix develop  # Enter development shell
cargo check  # Check compilation
cargo test   # Run tests
```