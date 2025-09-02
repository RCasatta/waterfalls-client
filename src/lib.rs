//! An extensible blocking/async Waterfalls client
//!
//! This library provides an extensible blocking and
//! async Waterfalls client to query Waterfalls's backend.
//!
//! The library provides the possibility to build a blocking
//! client using [`minreq`] and an async client using [`reqwest`].
//! The library supports communicating to Waterfalls via a proxy
//! and also using TLS (SSL) for secure communication.
//!
//!
//! ## Usage
//!
//! You can create a blocking client as follows:
//!
//! ```no_run
//! # #[cfg(feature = "blocking")]
//! # {
//! use waterfalls_client::Builder;
//! let builder = Builder::new("https://blockstream.info/testnet/api");
//! let blocking_client = builder.build_blocking();
//! # Ok::<(), waterfalls_client::Error>(());
//! # }
//! ```
//!
//! Here is an example of how to create an asynchronous client.
//!
//! ```no_run
//! # #[cfg(all(feature = "async", feature = "tokio"))]
//! # {
//! use waterfalls_client::Builder;
//! let builder = Builder::new("https://blockstream.info/testnet/api");
//! let async_client = builder.build_async();
//! # Ok::<(), waterfalls_client::Error>(());
//! # }
//! ```
//!
//! ## Features
//!
//! By default the library enables all features. To specify
//! specific features, set `default-features` to `false` in your `Cargo.toml`
//! and specify the features you want. This will look like this:
//!
//! `waterfalls-client = { version = "*", default-features = false, features =
//! ["blocking"] }`
//!
//! * `blocking` enables [`minreq`], the blocking client with proxy.
//! * `blocking-https` enables [`minreq`], the blocking client with proxy and TLS (SSL) capabilities
//!   using the default [`minreq`] backend.
//! * `blocking-https-rustls` enables [`minreq`], the blocking client with proxy and TLS (SSL)
//!   capabilities using the `rustls` backend.
//! * `blocking-https-native` enables [`minreq`], the blocking client with proxy and TLS (SSL)
//!   capabilities using the platform's native TLS backend (likely OpenSSL).
//! * `blocking-https-bundled` enables [`minreq`], the blocking client with proxy and TLS (SSL)
//!   capabilities using a bundled OpenSSL library backend.
//! * `async` enables [`reqwest`], the async client with proxy capabilities.
//! * `async-https` enables [`reqwest`], the async client with support for proxying and TLS (SSL)
//!   using the default [`reqwest`] TLS backend.
//! * `async-https-native` enables [`reqwest`], the async client with support for proxying and TLS
//!   (SSL) using the platform's native TLS backend (likely OpenSSL).
//! * `async-https-rustls` enables [`reqwest`], the async client with support for proxying and TLS
//!   (SSL) using the `rustls` TLS backend.
//! * `async-https-rustls-manual-roots` enables [`reqwest`], the async client with support for
//!   proxying and TLS (SSL) using the `rustls` TLS backend without using its the default root
//!   certificates.
//!
//! [`dont remove this line or cargo doc will break`]: https://example.com
#![cfg_attr(not(feature = "minreq"), doc = "[`minreq`]: https://docs.rs/minreq")]
#![cfg_attr(not(feature = "reqwest"), doc = "[`reqwest`]: https://docs.rs/reqwest")]
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::fmt;
use std::num::TryFromIntError;

#[cfg(feature = "async")]
pub use r#async::Sleeper;

pub mod api;
#[cfg(feature = "async")]
pub mod r#async;
#[cfg(feature = "blocking")]
pub mod blocking;

pub use api::*;
#[cfg(feature = "blocking")]
pub use blocking::BlockingClient;
#[cfg(feature = "async")]
pub use r#async::AsyncClient;

/// Response status codes for which the request may be retried.
pub const RETRYABLE_ERROR_CODES: [u16; 3] = [
    429, // TOO_MANY_REQUESTS
    500, // INTERNAL_SERVER_ERROR
    503, // SERVICE_UNAVAILABLE
];

/// Base backoff in milliseconds.
#[cfg(any(feature = "blocking", feature = "async"))]
const BASE_BACKOFF_MILLIS: std::time::Duration = std::time::Duration::from_millis(256);

/// Default max retries.
const DEFAULT_MAX_RETRIES: usize = 6;

#[derive(Debug, Clone)]
pub struct Builder {
    /// The URL of the Waterfalls server.
    pub base_url: String,
    /// Optional URL of the proxy to use to make requests to the Waterfalls server
    ///
    /// The string should be formatted as:
    /// `<protocol>://<user>:<password>@host:<port>`.
    ///
    /// Note that the format of this value and the supported protocols change
    /// slightly between the blocking version of the client (using `minreq`)
    /// and the async version (using `reqwest`). For more details check with
    /// the documentation of the two crates. Both of them are compiled with
    /// the `socks` feature enabled.
    ///
    /// The proxy is ignored when targeting `wasm32`.
    pub proxy: Option<String>,
    /// Socket timeout.
    pub timeout: Option<u64>,
    /// HTTP headers to set on every request made to Waterfalls server.
    pub headers: HashMap<String, String>,
    /// Max retries
    pub max_retries: usize,
}

impl Builder {
    /// Instantiate a new builder
    pub fn new(base_url: &str) -> Self {
        Builder {
            base_url: base_url.to_string(),
            proxy: None,
            timeout: None,
            headers: HashMap::new(),
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }

    /// Set the proxy of the builder
    pub fn proxy(mut self, proxy: &str) -> Self {
        self.proxy = Some(proxy.to_string());
        self
    }

    /// Set the timeout of the builder
    pub fn timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add a header to set on each request
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the maximum number of times to retry a request if the response status
    /// is one of [`RETRYABLE_ERROR_CODES`].
    pub fn max_retries(mut self, count: usize) -> Self {
        self.max_retries = count;
        self
    }

    /// Build a blocking client from builder
    #[cfg(feature = "blocking")]
    pub fn build_blocking(self) -> BlockingClient {
        BlockingClient::from_builder(self)
    }

    /// Build an asynchronous client from builder
    #[cfg(all(feature = "async", feature = "tokio"))]
    pub fn build_async(self) -> Result<AsyncClient, Error> {
        AsyncClient::from_builder(self)
    }

    /// Build an asynchronous client from builder where the returned client uses a
    /// user-defined [`Sleeper`].
    #[cfg(feature = "async")]
    pub fn build_async_with_sleeper<S: Sleeper>(self) -> Result<AsyncClient<S>, Error> {
        AsyncClient::from_builder(self)
    }
}

/// Errors that can happen during a request to `Waterfalls` servers.
#[derive(Debug)]
pub enum Error {
    /// Error during `minreq` HTTP request
    #[cfg(feature = "blocking")]
    Minreq(::minreq::Error),
    /// Error during reqwest HTTP request
    #[cfg(feature = "async")]
    Reqwest(::reqwest::Error),
    /// HTTP response error
    HttpResponse { status: u16, message: String },
    /// Invalid number returned
    Parsing(std::num::ParseIntError),
    /// Invalid status code, unable to convert to `u16`
    StatusCode(TryFromIntError),
    /// Invalid Bitcoin data returned
    BitcoinEncoding(bitcoin::consensus::encode::Error),
    /// Invalid hex data returned (attempting to create an array)
    HexToArray(bitcoin::hex::HexToArrayError),
    /// Invalid hex data returned (attempting to create a vector)
    HexToBytes(bitcoin::hex::HexToBytesError),
    /// Transaction not found
    TransactionNotFound(Txid),
    /// Block Header height not found
    HeaderHeightNotFound(u32),
    /// Block Header hash not found
    HeaderHashNotFound(BlockHash),
    /// Invalid HTTP Header name specified
    InvalidHttpHeaderName(String),
    /// Invalid HTTP Header value specified
    InvalidHttpHeaderValue(String),
    /// The server sent an invalid response
    InvalidResponse,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

macro_rules! impl_error {
    ( $from:ty, $to:ident ) => {
        impl_error!($from, $to, Error);
    };
    ( $from:ty, $to:ident, $impl_for:ty ) => {
        impl std::convert::From<$from> for $impl_for {
            fn from(err: $from) -> Self {
                <$impl_for>::$to(err)
            }
        }
    };
}

impl std::error::Error for Error {}
#[cfg(feature = "blocking")]
impl_error!(::minreq::Error, Minreq, Error);
#[cfg(feature = "async")]
impl_error!(::reqwest::Error, Reqwest, Error);
impl_error!(std::num::ParseIntError, Parsing, Error);
impl_error!(bitcoin::consensus::encode::Error, BitcoinEncoding, Error);
impl_error!(bitcoin::hex::HexToArrayError, HexToArray, Error);
impl_error!(bitcoin::hex::HexToBytesError, HexToBytes, Error);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::str::FromStr;

    #[test]
    fn test_builder() {
        let builder = Builder::new("https://waterfalls.example.com/api");
        assert_eq!(builder.base_url, "https://waterfalls.example.com/api");
        assert_eq!(builder.proxy, None);
        assert_eq!(builder.timeout, None);
        assert_eq!(builder.max_retries, DEFAULT_MAX_RETRIES);
        assert!(builder.headers.is_empty());
    }

    #[test]
    fn test_builder_with_proxy() {
        let builder =
            Builder::new("https://waterfalls.example.com/api").proxy("socks5://127.0.0.1:9050");
        assert_eq!(builder.proxy, Some("socks5://127.0.0.1:9050".to_string()));
    }

    #[test]
    fn test_builder_with_timeout() {
        let builder = Builder::new("https://waterfalls.example.com/api").timeout(30);
        assert_eq!(builder.timeout, Some(30));
    }

    #[test]
    fn test_builder_with_headers() {
        let builder = Builder::new("https://waterfalls.example.com/api")
            .header("User-Agent", "test-client")
            .header("Authorization", "Bearer token");

        let expected_headers: HashMap<String, String> = [
            ("User-Agent".to_string(), "test-client".to_string()),
            ("Authorization".to_string(), "Bearer token".to_string()),
        ]
        .into();

        assert_eq!(builder.headers, expected_headers);
    }

    #[test]
    fn test_builder_with_max_retries() {
        let builder = Builder::new("https://waterfalls.example.com/api").max_retries(10);
        assert_eq!(builder.max_retries, 10);
    }

    #[test]
    fn test_retryable_error_codes() {
        assert!(RETRYABLE_ERROR_CODES.contains(&429)); // TOO_MANY_REQUESTS
        assert!(RETRYABLE_ERROR_CODES.contains(&500)); // INTERNAL_SERVER_ERROR
        assert!(RETRYABLE_ERROR_CODES.contains(&503)); // SERVICE_UNAVAILABLE
        assert!(!RETRYABLE_ERROR_CODES.contains(&404)); // NOT_FOUND should not be retryable
    }

    #[test]
    fn test_v_serialization() {
        use crate::api::V;

        let undefined = V::Undefined;
        let vout = V::Vout(5);
        let vin = V::Vin(3);

        assert_eq!(undefined.raw(), 0);
        assert_eq!(vout.raw(), 5);
        assert_eq!(vin.raw(), -4); // -(3+1)

        assert_eq!(V::from_raw(0), V::Undefined);
        assert_eq!(V::from_raw(5), V::Vout(5));
        assert_eq!(V::from_raw(-4), V::Vin(3));
    }

    #[test]
    fn test_waterfall_response_is_empty() {
        use crate::api::{TxSeen, WaterfallResponse, V};
        use bitcoin::Txid;
        use std::collections::BTreeMap;

        // Empty response
        let empty_response = WaterfallResponse {
            txs_seen: BTreeMap::new(),
            page: 0,
            tip: None,
            tip_meta: None,
        };
        assert!(empty_response.is_empty());

        // Response with empty vectors
        let mut txs_seen = BTreeMap::new();
        txs_seen.insert("key1".to_string(), vec![vec![]]);
        let empty_vectors_response = WaterfallResponse {
            txs_seen,
            page: 0,
            tip: None,
            tip_meta: None,
        };
        assert!(empty_vectors_response.is_empty());

        // Response with actual transaction
        let mut txs_seen = BTreeMap::new();
        let tx_seen = TxSeen {
            txid: Txid::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            height: 100,
            block_hash: None,
            block_timestamp: None,
            v: V::Undefined,
        };
        txs_seen.insert("key1".to_string(), vec![vec![tx_seen]]);
        let non_empty_response = WaterfallResponse {
            txs_seen,
            page: 0,
            tip: None,
            tip_meta: None,
        };
        assert!(!non_empty_response.is_empty());
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_blocking_client_creation() {
        let builder = Builder::new("https://waterfalls.example.com/api");
        let _client = builder.build_blocking();
        // Just test that it doesn't panic
    }

    #[cfg(all(feature = "async", feature = "tokio"))]
    #[tokio::test]
    async fn test_async_client_creation() {
        let builder = Builder::new("https://waterfalls.example.com/api");
        let _client = builder.build_async();
        // Just test that it doesn't panic
    }
}
