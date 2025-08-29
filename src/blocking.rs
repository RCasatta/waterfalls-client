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

//! Waterfalls by way of `minreq` HTTP client.

use std::collections::HashMap;
use std::convert::TryFrom;
use std::str::FromStr;
use std::thread;

#[allow(unused_imports)]
use log::{debug, error, info, trace};

use minreq::{Proxy, Request, Response};

use bitcoin::consensus::{deserialize, serialize, Decodable};
use bitcoin::hex::{DisplayHex, FromHex};
use bitcoin::Address;
use bitcoin::{block::Header as BlockHeader, BlockHash, Transaction, Txid};

use crate::{Builder, Error, WaterfallResponse, BASE_BACKOFF_MILLIS, RETRYABLE_ERROR_CODES};

#[derive(Debug, Clone)]
pub struct BlockingClient {
    /// The URL of the Waterfalls server.
    url: String,
    /// The proxy is ignored when targeting `wasm32`.
    pub proxy: Option<String>,
    /// Socket timeout.
    pub timeout: Option<u64>,
    /// HTTP headers to set on every request made to Waterfalls server
    pub headers: HashMap<String, String>,
    /// Number of times to retry a request
    pub max_retries: usize,
}

impl BlockingClient {
    /// Build a blocking client from a [`Builder`]
    pub fn from_builder(builder: Builder) -> Self {
        Self {
            url: builder.base_url,
            proxy: builder.proxy,
            timeout: builder.timeout,
            headers: builder.headers,
            max_retries: builder.max_retries,
        }
    }

    /// Get the underlying base URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Perform a raw HTTP GET request with the given URI `path`.
    pub fn get_request(&self, path: &str) -> Result<Request, Error> {
        let mut request = minreq::get(format!("{}{}", self.url, path));

        if let Some(proxy) = &self.proxy {
            let proxy = Proxy::new(proxy.as_str())?;
            request = request.with_proxy(proxy);
        }

        if let Some(timeout) = &self.timeout {
            request = request.with_timeout(*timeout);
        }

        if !self.headers.is_empty() {
            for (key, value) in &self.headers {
                request = request.with_header(key, value);
            }
        }

        Ok(request)
    }

    fn get_opt_response<T: Decodable>(&self, path: &str) -> Result<Option<T>, Error> {
        match self.get_with_retry(path) {
            Ok(resp) if is_status_not_found(resp.status_code) => Ok(None),
            Ok(resp) if !is_status_ok(resp.status_code) => {
                let status = u16::try_from(resp.status_code).map_err(Error::StatusCode)?;
                let message = resp.as_str().unwrap_or_default().to_string();
                Err(Error::HttpResponse { status, message })
            }
            Ok(resp) => Ok(Some(deserialize::<T>(resp.as_bytes())?)),
            Err(e) => Err(e),
        }
    }

    fn get_response_hex<T: Decodable>(&self, path: &str) -> Result<T, Error> {
        match self.get_with_retry(path) {
            Ok(resp) if !is_status_ok(resp.status_code) => {
                let status = u16::try_from(resp.status_code).map_err(Error::StatusCode)?;
                let message = resp.as_str().unwrap_or_default().to_string();
                Err(Error::HttpResponse { status, message })
            }
            Ok(resp) => {
                let hex_str = resp.as_str().map_err(Error::Minreq)?;
                let hex_vec = Vec::from_hex(hex_str).unwrap();
                deserialize::<T>(&hex_vec).map_err(Error::BitcoinEncoding)
            }
            Err(e) => Err(e),
        }
    }

    fn get_response_json_with_query<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query_params: &[(&str, &str)],
    ) -> Result<T, Error> {
        let mut url = format!("{}{}", self.url, path);
        if !query_params.is_empty() {
            url.push('?');
            for (i, (key, value)) in query_params.iter().enumerate() {
                if i > 0 {
                    url.push('&');
                }
                // URL encode the key and value to handle special characters
                let encoded_key = urlencoding::encode(key);
                let encoded_value = urlencoding::encode(value);
                url.push_str(&format!("{encoded_key}={encoded_value}"));
            }
        }

        let mut request = minreq::get(&url);

        if let Some(proxy) = &self.proxy {
            let proxy = Proxy::new(proxy.as_str())?;
            request = request.with_proxy(proxy);
        }

        if let Some(timeout) = &self.timeout {
            request = request.with_timeout(*timeout);
        }

        if !self.headers.is_empty() {
            for (key, value) in &self.headers {
                request = request.with_header(key, value);
            }
        }

        match request.send() {
            Ok(resp) if !is_status_ok(resp.status_code) => {
                let status = u16::try_from(resp.status_code).map_err(Error::StatusCode)?;
                let message = resp.as_str().unwrap_or_default().to_string();
                Err(Error::HttpResponse { status, message })
            }
            Ok(resp) => Ok(resp.json::<T>()?),
            Err(e) => Err(Error::Minreq(e)),
        }
    }

    fn get_response_str(&self, path: &str) -> Result<String, Error> {
        match self.get_with_retry(path) {
            Ok(resp) if !is_status_ok(resp.status_code) => {
                let status = u16::try_from(resp.status_code).map_err(Error::StatusCode)?;
                let message = resp.as_str().unwrap_or_default().to_string();
                Err(Error::HttpResponse { status, message })
            }
            Ok(resp) => Ok(resp.as_str()?.to_string()),
            Err(e) => Err(e),
        }
    }

    /// Get a [`Transaction`] option given its [`Txid`]
    pub fn get_tx(&self, txid: &Txid) -> Result<Option<Transaction>, Error> {
        self.get_opt_response(&format!("/tx/{txid}/raw"))
    }

    /// Get a [`Transaction`] given its [`Txid`].
    pub fn get_tx_no_opt(&self, txid: &Txid) -> Result<Transaction, Error> {
        match self.get_tx(txid) {
            Ok(Some(tx)) => Ok(tx),
            Ok(None) => Err(Error::TransactionNotFound(*txid)),
            Err(e) => Err(e),
        }
    }

    /// Query the waterfalls endpoint with a descriptor
    pub fn waterfalls(&self, descriptor: &str) -> Result<WaterfallResponse, Error> {
        let path = "/v2/waterfalls";
        self.get_response_json_with_query(path, &[("descriptor", descriptor)])
    }

    /// Query the waterfalls endpoint with addresses
    pub fn waterfalls_addresses(&self, addresses: &[Address]) -> Result<WaterfallResponse, Error> {
        let addresses_str = addresses
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let path = "/v2/waterfalls";
        self.get_response_json_with_query(path, &[("addresses", &addresses_str)])
    }

    /// Query waterfalls with version-specific parameters
    pub fn waterfalls_version(
        &self,
        descriptor: &str,
        version: u8,
        page: Option<u32>,
        to_index: Option<u32>,
        utxo_only: bool,
    ) -> Result<WaterfallResponse, Error> {
        let path = format!("/v{version}/waterfalls");
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
        self.get_response_json_with_query(&path, &query_refs)
    }

    /// Get a [`BlockHeader`] given a particular block hash.
    pub fn get_header_by_hash(&self, block_hash: &BlockHash) -> Result<BlockHeader, Error> {
        self.get_response_hex(&format!("/block/{block_hash}/header"))
    }

    /// Get the server's public key for encryption
    pub fn server_recipient(&self) -> Result<String, Error> {
        self.get_response_str("/v1/server_recipient")
    }

    /// Get the server's address for message signing verification
    pub fn server_address(&self) -> Result<String, Error> {
        self.get_response_str("/v1/server_address")
    }

    /// Get time since last block with freshness indicator
    pub fn time_since_last_block(&self) -> Result<String, Error> {
        self.get_response_str("/v1/time_since_last_block")
    }

    /// Broadcast a [`Transaction`] to Waterfalls
    pub fn broadcast(&self, transaction: &Transaction) -> Result<(), Error> {
        let mut request = minreq::post(format!("{}/tx", self.url)).with_body(
            serialize(transaction)
                .to_lower_hex_string()
                .as_bytes()
                .to_vec(),
        );

        if let Some(proxy) = &self.proxy {
            let proxy = Proxy::new(proxy.as_str())?;
            request = request.with_proxy(proxy);
        }

        if let Some(timeout) = &self.timeout {
            request = request.with_timeout(*timeout);
        }

        match request.send() {
            Ok(resp) if !is_status_ok(resp.status_code) => {
                let status = u16::try_from(resp.status_code).map_err(Error::StatusCode)?;
                let message = resp.as_str().unwrap_or_default().to_string();
                Err(Error::HttpResponse { status, message })
            }
            Ok(_resp) => Ok(()),
            Err(e) => Err(Error::Minreq(e)),
        }
    }

    /// Get the [`BlockHash`] of the current blockchain tip.
    pub fn get_tip_hash(&self) -> Result<BlockHash, Error> {
        self.get_response_str("/blocks/tip/hash")
            .map(|s| BlockHash::from_str(s.as_str()).map_err(Error::HexToArray))?
    }

    /// Get the [`BlockHash`] of a specific block height
    pub fn get_block_hash(&self, block_height: u32) -> Result<BlockHash, Error> {
        self.get_response_str(&format!("/block-height/{block_height}"))
            .map(|s| BlockHash::from_str(s.as_str()).map_err(Error::HexToArray))?
    }

    /// Get transaction history for the specified address in Esplora-compatible format
    pub fn get_address_txs(&self, address: &Address) -> Result<String, Error> {
        let path = format!("/address/{address}/txs");
        self.get_response_str(&path)
    }

    /// Sends a GET request to the given `url`, retrying failed attempts
    /// for retryable error codes until max retries hit.
    fn get_with_retry(&self, url: &str) -> Result<Response, Error> {
        let mut delay = BASE_BACKOFF_MILLIS;
        let mut attempts = 0;

        loop {
            match self.get_request(url)?.send()? {
                resp if attempts < self.max_retries && is_status_retryable(resp.status_code) => {
                    thread::sleep(delay);
                    attempts += 1;
                    delay *= 2;
                }
                resp => return Ok(resp),
            }
        }
    }
}

fn is_status_ok(status: i32) -> bool {
    status == 200
}

fn is_status_not_found(status: i32) -> bool {
    status == 404
}

fn is_status_retryable(status: i32) -> bool {
    let status = status as u16;
    RETRYABLE_ERROR_CODES.contains(&status)
}
