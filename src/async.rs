// Bitcoin Dev Kit
// Written in 2020 by Alekos Filini <alekos.filini@gmail.com>
//
// Copyright (c) 2020-2021 Bitcoin Dev Kit Developers
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

//! Waterfalls by way of `reqwest` HTTP client.

use std::marker::PhantomData;
use std::str::FromStr;

use bitcoin::consensus::{deserialize, serialize, Decodable, Encodable};
use bitcoin::hex::{DisplayHex, FromHex};
use bitcoin::Address;
use bitcoin::{block::Header as BlockHeader, BlockHash, Transaction, Txid};

#[allow(unused_imports)]
use log::{debug, error, info, trace};

use reqwest::{header, Client, Response};

use crate::{Builder, Error, WaterfallResponse, BASE_BACKOFF_MILLIS, RETRYABLE_ERROR_CODES};

#[derive(Debug, Clone)]
pub struct AsyncClient<S = DefaultSleeper> {
    /// The URL of the Waterfalls Server.
    url: String,
    /// The inner [`reqwest::Client`] to make HTTP requests.
    client: Client,
    /// Number of times to retry a request
    max_retries: usize,

    /// Marker for the type of sleeper used
    marker: PhantomData<S>,
}

impl<S: Sleeper> AsyncClient<S> {
    /// Build an async client from a builder
    pub fn from_builder(builder: Builder) -> Result<Self, Error> {
        let mut client_builder = Client::builder();

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(proxy) = &builder.proxy {
            client_builder = client_builder.proxy(reqwest::Proxy::all(proxy)?);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(timeout) = builder.timeout {
            client_builder = client_builder.timeout(core::time::Duration::from_secs(timeout));
        }

        if !builder.headers.is_empty() {
            let mut headers = header::HeaderMap::new();
            for (k, v) in &builder.headers {
                let header_name = header::HeaderName::from_lowercase(k.to_lowercase().as_bytes())
                    .map_err(|_| Error::InvalidHttpHeaderName(k.clone()))?;
                let header_value = header::HeaderValue::from_str(v)
                    .map_err(|_| Error::InvalidHttpHeaderValue(v.clone()))?;
                headers.insert(header_name, header_value);
            }
            client_builder = client_builder.default_headers(headers);
        }

        Ok(AsyncClient {
            url: builder.base_url,
            client: client_builder.build()?,
            max_retries: builder.max_retries,
            marker: PhantomData,
        })
    }

    pub fn from_client(url: String, client: Client) -> Self {
        AsyncClient {
            url,
            client,
            max_retries: crate::DEFAULT_MAX_RETRIES,
            marker: PhantomData,
        }
    }

    /// Make an HTTP GET request to given URL, deserializing to any `T` that
    /// implement [`bitcoin::consensus::Decodable`].
    ///
    /// It should be used when requesting Waterfalls endpoints that can be directly
    /// deserialized to native `rust-bitcoin` types, which implements
    /// [`bitcoin::consensus::Decodable`] from `&[u8]`.
    ///
    /// # Errors
    ///
    /// This function will return an error either from the HTTP client, or the
    /// [`bitcoin::consensus::Decodable`] deserialization.
    async fn get_response<T: Decodable>(&self, path: &str) -> Result<T, Error> {
        let url = format!("{}{}", self.url, path);
        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(Error::HttpResponse {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        Ok(deserialize::<T>(&response.bytes().await?)?)
    }

    /// Make an HTTP GET request to given URL, deserializing to `Option<T>`.
    ///
    /// It uses [`AsyncWaterfallsClient::get_response`] internally.
    ///
    /// See [`AsyncWaterfallsClient::get_response`] above for full documentation.
    async fn get_opt_response<T: Decodable>(&self, path: &str) -> Result<Option<T>, Error> {
        match self.get_response::<T>(path).await {
            Ok(res) => Ok(Some(res)),
            Err(Error::HttpResponse { status: 404, .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Make an HTTP GET request to given URL with query parameters, deserializing to any `T` that
    /// implements [`serde::de::DeserializeOwned`].
    async fn get_response_json_with_query<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query_params: &[(&str, &str)],
    ) -> Result<T, Error> {
        let url = format!("{}{}", self.url, path);
        let mut request = self.client.get(&url);
        for (key, value) in query_params {
            request = request.query(&[(key, value)]);
        }
        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(Error::HttpResponse {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response.json::<T>().await.map_err(Error::Reqwest)
    }

    /// Make an HTTP GET request to given URL, deserializing to any `T` that
    /// implements [`bitcoin::consensus::Decodable`].
    ///
    /// It should be used when requesting Waterfalls endpoints that are expected
    /// to return a hex string decodable to native `rust-bitcoin` types which
    /// implement [`bitcoin::consensus::Decodable`] from `&[u8]`.
    ///
    /// # Errors
    ///
    /// This function will return an error either from the HTTP client, or the
    /// [`bitcoin::consensus::Decodable`] deserialization.
    async fn get_response_hex<T: Decodable>(&self, path: &str) -> Result<T, Error> {
        let url = format!("{}{}", self.url, path);
        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(Error::HttpResponse {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        let hex_str = response.text().await?;
        Ok(deserialize(&Vec::from_hex(&hex_str)?)?)
    }

    /// Make an HTTP GET request to given URL, deserializing to `String`.
    ///
    /// It should be used when requesting Waterfalls endpoints that can return
    /// `String` formatted data that can be parsed downstream.
    ///
    /// # Errors
    ///
    /// This function will return an error either from the HTTP client.
    async fn get_response_text(&self, path: &str) -> Result<String, Error> {
        let url = format!("{}{}", self.url, path);
        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(Error::HttpResponse {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        Ok(response.text().await?)
    }

    /// Make an HTTP POST request to given URL, serializing from any `T` that
    /// implement [`bitcoin::consensus::Encodable`].
    ///
    /// It should be used when requesting Waterfalls endpoints that expected a
    /// native bitcoin type serialized with [`bitcoin::consensus::Encodable`].
    ///
    /// # Errors
    ///
    /// This function will return an error either from the HTTP client, or the
    /// [`bitcoin::consensus::Encodable`] serialization.
    async fn post_request_hex<T: Encodable>(&self, path: &str, body: T) -> Result<(), Error> {
        let url = format!("{}{}", self.url, path);
        let body = serialize::<T>(&body).to_lower_hex_string();

        let response = self.client.post(url).body(body).send().await?;

        if !response.status().is_success() {
            return Err(Error::HttpResponse {
                status: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        Ok(())
    }

    /// Get a [`Transaction`] option given its [`Txid`]
    pub async fn get_tx(&self, txid: &Txid) -> Result<Option<Transaction>, Error> {
        self.get_opt_response(&format!("/tx/{txid}/raw")).await
    }

    /// Get a [`Transaction`] given its [`Txid`].
    pub async fn get_tx_no_opt(&self, txid: &Txid) -> Result<Transaction, Error> {
        match self.get_tx(txid).await {
            Ok(Some(tx)) => Ok(tx),
            Ok(None) => Err(Error::TransactionNotFound(*txid)),
            Err(e) => Err(e),
        }
    }

    /// Query the waterfalls endpoint with a descriptor
    pub async fn waterfalls(&self, descriptor: &str) -> Result<WaterfallResponse, Error> {
        let path = "/v2/waterfalls";
        self.get_response_json_with_query(path, &[("descriptor", descriptor)])
            .await
    }

    /// Query the waterfalls endpoint with addresses
    pub async fn waterfalls_addresses(
        &self,
        addresses: &[Address],
    ) -> Result<WaterfallResponse, Error> {
        let addresses_str = addresses
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let path = "/v2/waterfalls";
        self.get_response_json_with_query(path, &[("addresses", &addresses_str)])
            .await
    }

    /// Query waterfalls with version-specific parameters
    pub async fn waterfalls_version(
        &self,
        descriptor: &str,
        version: u8,
        page: Option<u32>,
        to_index: Option<u32>,
        utxo_only: bool,
    ) -> Result<WaterfallResponse, Error> {
        let path = format!("/v{}/waterfalls", version);
        let mut query_params = vec![
            ("descriptor", descriptor.to_string()),
            ("utxo_only", utxo_only.to_string()),
        ];

        if let Some(page) = page {
            query_params.push(("page", page.to_string()));
        }
        if let Some(to_index) = to_index {
            query_params.push(("to_index", to_index.to_string()));
        }

        let query_refs: Vec<(&str, &str)> =
            query_params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        self.get_response_json_with_query(&path, &query_refs).await
    }

    /// Get a [`BlockHeader`] given a particular block hash.
    pub async fn get_header_by_hash(&self, block_hash: &BlockHash) -> Result<BlockHeader, Error> {
        self.get_response_hex(&format!("/block/{block_hash}/header"))
            .await
    }

    /// Get the server's public key for encryption
    pub async fn server_recipient(&self) -> Result<String, Error> {
        self.get_response_text("/v1/server_recipient").await
    }

    /// Get the server's address for message signing verification
    pub async fn server_address(&self) -> Result<String, Error> {
        self.get_response_text("/v1/server_address").await
    }

    /// Get time since last block with freshness indicator
    pub async fn time_since_last_block(&self) -> Result<String, Error> {
        self.get_response_text("/v1/time_since_last_block").await
    }

    /// Broadcast a [`Transaction`] to Waterfalls
    pub async fn broadcast(&self, transaction: &Transaction) -> Result<(), Error> {
        self.post_request_hex("/tx", transaction).await
    }

    /// Get the [`BlockHash`] of the current blockchain tip.
    pub async fn get_tip_hash(&self) -> Result<BlockHash, Error> {
        self.get_response_text("/blocks/tip/hash")
            .await
            .map(|block_hash| BlockHash::from_str(&block_hash).map_err(Error::HexToArray))?
    }

    /// Get the [`BlockHash`] of a specific block height
    pub async fn get_block_hash(&self, block_height: u32) -> Result<BlockHash, Error> {
        self.get_response_text(&format!("/block-height/{block_height}"))
            .await
            .map(|block_hash| BlockHash::from_str(&block_hash).map_err(Error::HexToArray))?
    }

    /// Get transaction history for the specified address in Esplora-compatible format
    pub async fn get_address_txs(&self, address: &Address) -> Result<String, Error> {
        let path = format!("/address/{address}/txs");
        self.get_response_text(&path).await
    }

    /// Get the underlying base URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the underlying [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Sends a GET request to the given `url`, retrying failed attempts
    /// for retryable error codes until max retries hit.
    async fn get_with_retry(&self, url: &str) -> Result<Response, Error> {
        let mut delay = BASE_BACKOFF_MILLIS;
        let mut attempts = 0;

        loop {
            match self.client.get(url).send().await? {
                resp if attempts < self.max_retries && is_status_retryable(resp.status()) => {
                    S::sleep(delay).await;
                    attempts += 1;
                    delay *= 2;
                }
                resp => return Ok(resp),
            }
        }
    }
}

fn is_status_retryable(status: reqwest::StatusCode) -> bool {
    RETRYABLE_ERROR_CODES.contains(&status.as_u16())
}

pub trait Sleeper: 'static {
    type Sleep: std::future::Future<Output = ()>;
    fn sleep(dur: std::time::Duration) -> Self::Sleep;
}

#[derive(Debug, Clone, Copy)]
pub struct DefaultSleeper;

#[cfg(any(test, feature = "tokio"))]
impl Sleeper for DefaultSleeper {
    type Sleep = tokio::time::Sleep;

    fn sleep(dur: std::time::Duration) -> Self::Sleep {
        tokio::time::sleep(dur)
    }
}
