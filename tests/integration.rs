//! Integration tests for waterfalls-client
//!
//! These tests verify that the waterfalls-client works correctly with
//! an actual waterfalls server instance.

#[cfg(any(feature = "blocking", feature = "async"))]
use waterfalls_client::{Builder, WaterfallResponse};

#[cfg(any(feature = "blocking", feature = "async"))]
use bitcoin::Network;

#[cfg(any(feature = "blocking", feature = "async"))]
async fn launch_test_env() -> waterfalls::test_env::TestEnv {
    let exe = std::env::var("BITCOIND_EXEC").expect("BITCOIND_EXEC must be set");
    waterfalls::test_env::launch(exe, waterfalls::be::Family::Bitcoin).await
}

#[cfg(any(feature = "blocking", feature = "async"))]
// Helper functions to convert between waterfalls types and bitcoin types
fn convert_txid(waterfalls_txid: waterfalls::be::Txid) -> bitcoin::Txid {
    waterfalls_txid.bitcoin()
}

#[cfg(any(feature = "blocking", feature = "async"))]
fn convert_transaction(
    waterfalls_tx: &waterfalls::be::Transaction,
) -> Option<&bitcoin::Transaction> {
    match waterfalls_tx {
        waterfalls::be::Transaction::Bitcoin(tx) => Some(tx),
        waterfalls::be::Transaction::Elements(_) => None,
    }
}

#[cfg(any(feature = "blocking", feature = "async"))]
fn convert_address(waterfalls_addr: &waterfalls::be::Address) -> Option<&bitcoin::Address> {
    match waterfalls_addr {
        waterfalls::be::Address::Bitcoin(addr) => Some(addr),
        waterfalls::be::Address::Elements(_) => None,
    }
}

#[cfg(feature = "blocking")]
#[test]
fn test_get_tx_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Create a transaction to test with
    let address = test_env.get_new_address(None);
    let txid = test_env.send_to(&address, 10000);
    rt.block_on(test_env.node_generate(1));

    // Convert waterfalls txid to bitcoin txid
    let bitcoin_txid = convert_txid(txid);

    // Test blocking client can retrieve the transaction
    let tx_blocking = blocking_client.get_tx(&bitcoin_txid).unwrap();

    assert!(tx_blocking.is_some());

    if let Some(tx) = tx_blocking {
        assert_eq!(tx.compute_txid(), bitcoin_txid);
    }

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_get_tx_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Create a transaction to test with
    let address = test_env.get_new_address(None);
    let txid = test_env.send_to(&address, 10000);
    test_env.node_generate(1).await;

    // Convert waterfalls txid to bitcoin txid
    let bitcoin_txid = convert_txid(txid);

    // Test async client can retrieve the transaction
    let tx_async = async_client.get_tx(&bitcoin_txid).await.unwrap();

    assert!(tx_async.is_some());

    if let Some(tx) = tx_async {
        assert_eq!(tx.compute_txid(), bitcoin_txid);
    }

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_get_tx_no_opt_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Create a transaction to test with
    let address = test_env.get_new_address(None);
    let txid = test_env.send_to(&address, 10000);
    rt.block_on(test_env.node_generate(1));

    // Convert waterfalls txid to bitcoin txid
    let bitcoin_txid = convert_txid(txid);

    // Test blocking client can retrieve the transaction
    let tx_blocking = blocking_client.get_tx_no_opt(&bitcoin_txid).unwrap();

    assert_eq!(tx_blocking.compute_txid(), bitcoin_txid);

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_get_tx_no_opt_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Create a transaction to test with
    let address = test_env.get_new_address(None);
    let txid = test_env.send_to(&address, 10000);
    test_env.node_generate(1).await;

    // Convert waterfalls txid to bitcoin txid
    let bitcoin_txid = convert_txid(txid);

    // Test async client can retrieve the transaction
    let tx_async = async_client.get_tx_no_opt(&bitcoin_txid).await.unwrap();

    assert_eq!(tx_async.compute_txid(), bitcoin_txid);

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_get_tip_hash_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    let tip_hash_blocking = blocking_client.get_tip_hash().unwrap();

    // Assert we got a valid block hash
    assert!(!tip_hash_blocking.to_string().is_empty());

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_get_tip_hash_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    let tip_hash_async = async_client.get_tip_hash().await.unwrap();

    // Assert we got a valid block hash
    assert!(!tip_hash_async.to_string().is_empty());

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_get_block_hash_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Get block hash at a specific height
    let block_hash_blocking = blocking_client.get_block_hash(0).unwrap();

    // Assert we got a valid block hash
    assert!(!block_hash_blocking.to_string().is_empty());

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_get_block_hash_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Get block hash at a specific height
    let block_hash_async = async_client.get_block_hash(0).await.unwrap();

    // Assert we got a valid block hash
    assert!(!block_hash_async.to_string().is_empty());

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_get_header_by_hash_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Get the genesis block hash and header
    let block_hash = blocking_client.get_block_hash(0).unwrap();
    let header_blocking = blocking_client.get_header_by_hash(&block_hash).unwrap();

    // Assert we got a valid header (check version is non-zero)
    assert_ne!(header_blocking.version.to_consensus(), 0);

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_get_header_by_hash_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Get the genesis block hash and header
    let block_hash = async_client.get_block_hash(0).await.unwrap();
    let header_async = async_client.get_header_by_hash(&block_hash).await.unwrap();

    // Assert we got a valid header (check version is non-zero)
    assert_ne!(header_async.version.to_consensus(), 0);

    test_env.shutdown().await;
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_broadcast() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Create and sign a transaction
    let tx = test_env.create_self_transanction();
    let signed_tx = test_env.sign_raw_transanction_with_wallet(&tx);

    // Convert waterfalls transaction to bitcoin transaction
    let bitcoin_tx = convert_transaction(&signed_tx)
        .expect("Expected Bitcoin transaction from test environment");

    // Test broadcasting with async client
    async_client.broadcast(bitcoin_tx).await.unwrap();

    // Verify the transaction was broadcast by trying to get it
    let tx_txid = bitcoin_tx.compute_txid();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let retrieved_tx = async_client.get_tx(&tx_txid).await.unwrap();
    assert!(retrieved_tx.is_some());

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_waterfalls_endpoint_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Test descriptor from the waterfalls integration test
    let descriptor = "wpkh(tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*)";

    // Test waterfalls endpoint
    let result_blocking = blocking_client.waterfalls(descriptor).unwrap();

    assert_eq!(result_blocking.page, 0);
    assert!(result_blocking.tip.is_none());
    assert!(result_blocking.tip_meta.is_some());

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_waterfalls_endpoint_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Test descriptor from the waterfalls integration test
    let descriptor = "wpkh(tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*)";

    // Test waterfalls endpoint
    let result_async = async_client.waterfalls(descriptor).await.unwrap();

    assert_eq!(result_async.page, 0);
    assert!(result_async.tip.is_none());
    assert!(result_async.tip_meta.is_some());

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_waterfalls_addresses_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Create a test address
    let waterfalls_address = test_env.get_new_address(None);
    let bitcoin_address = convert_address(&waterfalls_address)
        .expect("Expected Bitcoin address from test environment");
    let addresses = vec![bitcoin_address.clone()];

    // Send some funds to the address
    let _txid = test_env.send_to(&waterfalls_address, 10000);
    rt.block_on(test_env.node_generate(1));

    // Test waterfalls_addresses endpoint
    let result_blocking = blocking_client.waterfalls_addresses(&addresses).unwrap();

    assert!(!result_blocking.is_empty());

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_waterfalls_addresses_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Create a test address
    let waterfalls_address = test_env.get_new_address(None);
    let bitcoin_address = convert_address(&waterfalls_address)
        .expect("Expected Bitcoin address from test environment");
    let addresses = vec![bitcoin_address.clone()];

    // Send some funds to the address
    let _txid = test_env.send_to(&waterfalls_address, 10000);
    test_env.node_generate(1).await;

    // Test waterfalls_addresses endpoint
    let result_async = async_client.waterfalls_addresses(&addresses).await.unwrap();

    assert!(!result_async.is_empty());

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_waterfalls_version_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    let descriptor = "wpkh(tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*)";

    // Test waterfalls_version endpoint with various parameters
    let result_blocking = blocking_client
        .waterfalls_version(descriptor, 2, None, None, false)
        .unwrap();

    assert_eq!(result_blocking.page, 0);

    // Test with utxo_only = true
    let result_utxo_blocking = blocking_client
        .waterfalls_version(descriptor, 2, None, None, true)
        .unwrap();

    assert_eq!(result_utxo_blocking.page, 0);

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_waterfalls_version_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    let descriptor = "wpkh(tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*)";

    // Test waterfalls_version endpoint with various parameters
    let result_async = async_client
        .waterfalls_version(descriptor, 2, None, None, false)
        .await
        .unwrap();

    assert_eq!(result_async.page, 0);

    // Test with utxo_only = true
    let result_utxo_async = async_client
        .waterfalls_version(descriptor, 2, None, None, true)
        .await
        .unwrap();

    assert_eq!(result_utxo_async.page, 0);

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_server_info_endpoints_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Test server_recipient endpoint
    let recipient_blocking = blocking_client.server_recipient().unwrap();
    assert!(!recipient_blocking.is_empty());

    // Test server_address endpoint
    let address_blocking = blocking_client.server_address().unwrap();
    assert!(!address_blocking.is_empty());

    // Test time_since_last_block endpoint
    let time_blocking = blocking_client.time_since_last_block().unwrap();
    assert!(!time_blocking.is_empty());

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_server_info_endpoints_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Test server_recipient endpoint
    let recipient_async = async_client.server_recipient().await.unwrap();
    assert!(!recipient_async.is_empty());

    // Test server_address endpoint
    let address_async = async_client.server_address().await.unwrap();
    assert!(!address_async.is_empty());

    // Test time_since_last_block endpoint
    let time_async = async_client.time_since_last_block().await.unwrap();
    assert!(!time_async.is_empty());

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_get_address_txs_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Create a test address and send funds to it
    let waterfalls_address = test_env.get_new_address(None);
    let bitcoin_address = convert_address(&waterfalls_address)
        .expect("Expected Bitcoin address from test environment");
    let waterfalls_txid = test_env.send_to(&waterfalls_address, 10000);
    let bitcoin_txid = convert_txid(waterfalls_txid);
    rt.block_on(test_env.node_generate(1));

    // Test get_address_txs endpoint
    let txs_blocking = blocking_client.get_address_txs(bitcoin_address).unwrap();

    assert!(txs_blocking.contains(&bitcoin_txid.to_string()));

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_get_address_txs_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Create a test address and send funds to it
    let waterfalls_address = test_env.get_new_address(None);
    let bitcoin_address = convert_address(&waterfalls_address)
        .expect("Expected Bitcoin address from test environment");
    let waterfalls_txid = test_env.send_to(&waterfalls_address, 10000);
    let bitcoin_txid = convert_txid(waterfalls_txid);
    test_env.node_generate(1).await;

    // Test get_address_txs endpoint
    let txs_async = async_client.get_address_txs(bitcoin_address).await.unwrap();

    assert!(txs_async.contains(&bitcoin_txid.to_string()));

    test_env.shutdown().await;
}

#[cfg(feature = "blocking")]
#[test]
fn test_client_with_headers_blocking() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let test_env = rt.block_on(launch_test_env());
    let url = test_env.base_url();

    // Test client with custom headers
    let headers = std::collections::HashMap::from([
        (
            "User-Agent".to_string(),
            "waterfalls-client-test".to_string(),
        ),
        ("X-Test-Header".to_string(), "test-value".to_string()),
    ]);

    let mut builder = Builder::new(url);
    for (key, value) in headers {
        builder = builder.header(&key, &value);
    }

    let blocking_client = builder.build_blocking();

    // Test that the client still works with custom headers
    let tip_hash_blocking = blocking_client.get_tip_hash().unwrap();

    // Assert we got a valid block hash
    assert!(!tip_hash_blocking.to_string().is_empty());

    rt.block_on(test_env.shutdown());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_client_with_headers_async() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    // Test client with custom headers
    let headers = std::collections::HashMap::from([
        (
            "User-Agent".to_string(),
            "waterfalls-client-test".to_string(),
        ),
        ("X-Test-Header".to_string(), "test-value".to_string()),
    ]);

    let mut builder = Builder::new(url);
    for (key, value) in headers {
        builder = builder.header(&key, &value);
    }

    let async_client = builder.build_async().unwrap();

    // Test that the client still works with custom headers
    let tip_hash_async = async_client.get_tip_hash().await.unwrap();

    // Assert we got a valid block hash
    assert!(!tip_hash_async.to_string().is_empty());

    test_env.shutdown().await;
}

//
// Production Tests using real URLs and descriptors
//

#[cfg(any(feature = "blocking", feature = "async"))]
fn get_production_url(network: Network) -> Option<&'static str> {
    match network {
        Network::Bitcoin => Some("https://waterfalls.liquidwebwallet.org/bitcoin/api"),
        Network::Signet => Some("https://waterfalls.liquidwebwallet.org/bitcoinsignet/api"),
        _ => None,
    }
}

#[cfg(any(feature = "blocking", feature = "async"))]
fn get_production_descriptor(network: Network) -> Option<&'static str> {
    match network {
        Network::Bitcoin => Some("sh(wpkh(xpub6C6nQwHaWbSrzs5tZ1q7m5R9cPK9eYpNMFesiXsYrgc1P8bvLLAet9JfHjYXKjToD8cBRswJXXbbFpXgwsswVPAZzKMa1jUp2kVkGVUaJa7/<0;1>/*))"),
        Network::Signet => Some("tr(tpubDDh1wUM29wsoJnHomNYrEwhGainWHUSzErfNrsZKiCjQWWUjFLwhtAqWvGUKc4oESXqcGKdbPDv7fBDsPHPYitNuGNrJ9BKrW1GPxUyiUUb/<0;1>/*)"),
        _ => None,
    }
}

#[cfg(any(feature = "blocking", feature = "async"))]
fn assert_result(response: &WaterfallResponse, min_txseens: usize) {
    // Count total transactions across all keys and nested vectors
    let total_transactions = response
        .txs_seen
        .values()
        .flat_map(|v| v.iter())
        .map(|inner_vec| inner_vec.len())
        .sum::<usize>();

    // Assert we have a valid tip
    assert!(
        response.tip_meta.is_some(),
        "Response should have a tip meta"
    );
    assert!(response.tip.is_none(), "Response should not have a tip");

    // Assert we have at least the minimum number of transactions
    assert!(
        total_transactions >= min_txseens,
        "Expected at least {min_txseens} transactions, but found {total_transactions}"
    );

    println!(
        "Found {} transactions (minimum expected: {}), tip: {:?}",
        total_transactions, min_txseens, response.tip
    );
}

#[cfg(feature = "blocking-https")]
fn test_blocking(network: Network, min_txseens: usize) {
    let url = get_production_url(network).expect("URL not found for network");
    let descriptor = get_production_descriptor(network).expect("Descriptor not found for network");

    let builder = Builder::new(url);
    let blocking_client = builder.build_blocking();

    // Test waterfalls endpoint with production descriptor
    let result = blocking_client.waterfalls(descriptor).unwrap();
    println!("Blocking result: {:?}", result);

    // Assert the response has the expected number of transactions
    assert_result(&result, min_txseens);
    println!("Blocking {network:?} test passed");
}

#[cfg(feature = "async")]
async fn test_async(network: Network, min_txseens: usize) {
    let url = get_production_url(network).expect("URL not found for network");
    let descriptor = get_production_descriptor(network).expect("Descriptor not found for network");

    let builder = Builder::new(url);
    let async_client = builder.build_async().unwrap();

    // Test waterfalls endpoint with production descriptor
    let result = async_client.waterfalls(descriptor).await.unwrap();

    // Assert the response has the expected number of transactions
    assert_result(&result, min_txseens);
    println!("Async {network:?} test passed");
}

#[cfg(feature = "blocking-https")]
#[test]
#[ignore]
fn test_blocking_mainnet() {
    test_blocking(Network::Bitcoin, 28);
}

#[cfg(feature = "blocking-https")]
#[test]
#[ignore]
fn test_blocking_signet() {
    test_blocking(Network::Signet, 5);
}

#[cfg(feature = "async")]
#[tokio::test]
#[ignore]
async fn test_async_mainnet() {
    test_async(Network::Bitcoin, 28).await;
}

#[cfg(feature = "async")]
#[tokio::test]
#[ignore]
async fn test_async_signet() {
    test_async(Network::Signet, 5).await;
}
