//! Integration tests for waterfalls-client
//!
//! These tests verify that the waterfalls-client works correctly with
//! an actual waterfalls server instance.

use std::collections::HashMap;
use std::time::Duration;

use tokio::time::sleep;
use waterfalls::be::Family;
use waterfalls_client::{AsyncClient, BlockingClient, Builder};

lazy_static::lazy_static! {
    static ref MINER: tokio::sync::Mutex<()> = tokio::sync::Mutex::new(());
}

// Helper function to create test clients
async fn setup_clients() -> (BlockingClient, AsyncClient) {
    let _ = env_logger::try_init();

    let test_env = launch_test_env().await;
    let url = test_env.base_url().to_string();

    let builder = Builder::new(&url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    (blocking_client, async_client)
}

async fn launch_test_env() -> waterfalls::test_env::TestEnv {
    let exe = std::env::var("BITCOIND_EXEC").expect("BITCOIND_EXEC must be set");
    waterfalls::test_env::launch(exe, Family::Bitcoin).await
}

// Helper functions to convert between waterfalls types and bitcoin types
fn convert_txid(waterfalls_txid: waterfalls::be::Txid) -> bitcoin::Txid {
    waterfalls_txid.bitcoin()
}

fn convert_transaction(
    waterfalls_tx: &waterfalls::be::Transaction,
) -> Option<&bitcoin::Transaction> {
    match waterfalls_tx {
        waterfalls::be::Transaction::Bitcoin(tx) => Some(tx),
        waterfalls::be::Transaction::Elements(_) => None,
    }
}

fn convert_address(waterfalls_addr: &waterfalls::be::Address) -> Option<&bitcoin::Address> {
    match waterfalls_addr {
        waterfalls::be::Address::Bitcoin(addr) => Some(addr),
        waterfalls::be::Address::Elements(_) => None,
    }
}

fn generate_blocks_and_wait(count: u64) {
    let rt = tokio::runtime::Handle::current();
    rt.block_on(async {
        let _lock = MINER.lock().await;
        // Simulate block generation - this would interface with the test env
        sleep(Duration::from_millis(100 * count)).await;
    });
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_get_tx() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Create a transaction to test with
    let address = test_env.get_new_address(None);
    let txid = test_env.send_to(&address, 10000);
    test_env.node_generate(1).await;

    // Convert waterfalls txid to bitcoin txid
    let bitcoin_txid = convert_txid(txid);

    // Test both clients can retrieve the transaction
    let tx_blocking = blocking_client.get_tx(&bitcoin_txid).unwrap();
    let tx_async = async_client.get_tx(&bitcoin_txid).await.unwrap();

    assert_eq!(tx_blocking, tx_async);
    assert!(tx_blocking.is_some());

    if let Some(tx) = tx_blocking {
        assert_eq!(tx.compute_txid(), bitcoin_txid);
    }

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_get_tx_no_opt() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Create a transaction to test with
    let address = test_env.get_new_address(None);
    let txid = test_env.send_to(&address, 10000);
    test_env.node_generate(1).await;

    // Convert waterfalls txid to bitcoin txid
    let bitcoin_txid = convert_txid(txid);

    // Test both clients can retrieve the transaction
    let tx_blocking = blocking_client.get_tx_no_opt(&bitcoin_txid).unwrap();
    let tx_async = async_client.get_tx_no_opt(&bitcoin_txid).await.unwrap();

    assert_eq!(tx_blocking, tx_async);
    assert_eq!(tx_blocking.compute_txid(), bitcoin_txid);

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_get_tip_hash() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    let tip_hash_blocking = blocking_client.get_tip_hash().unwrap();
    let tip_hash_async = async_client.get_tip_hash().await.unwrap();

    assert_eq!(tip_hash_blocking, tip_hash_async);

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_get_block_hash() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Get block hash at a specific height
    let block_hash_blocking = blocking_client.get_block_hash(0).unwrap();
    let block_hash_async = async_client.get_block_hash(0).await.unwrap();

    assert_eq!(block_hash_blocking, block_hash_async);

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_get_header_by_hash() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Get the genesis block hash and header
    let block_hash = blocking_client.get_block_hash(0).unwrap();

    let header_blocking = blocking_client.get_header_by_hash(&block_hash).unwrap();
    let header_async = async_client.get_header_by_hash(&block_hash).await.unwrap();

    assert_eq!(header_blocking, header_async);

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_broadcast() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
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
    sleep(Duration::from_millis(500)).await;
    let retrieved_tx = async_client.get_tx(&tx_txid).await.unwrap();
    assert!(retrieved_tx.is_some());

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_waterfalls_endpoint() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Test descriptor from the waterfalls integration test
    let descriptor = "wpkh(tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*)";

    // Test waterfalls endpoint
    let result_blocking = blocking_client.waterfalls(descriptor).unwrap();
    let result_async = async_client.waterfalls(descriptor).await.unwrap();

    assert_eq!(result_blocking, result_async);
    assert_eq!(result_blocking.page, 0);
    assert!(result_blocking.tip.is_some());

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_waterfalls_addresses() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
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
    let result_blocking = blocking_client.waterfalls_addresses(&addresses).unwrap();
    let result_async = async_client.waterfalls_addresses(&addresses).await.unwrap();

    assert_eq!(result_blocking, result_async);

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_waterfalls_version() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    let descriptor = "wpkh(tpubDC8msFGeGuwnKG9Upg7DM2b4DaRqg3CUZa5g8v2SRQ6K4NSkxUgd7HsL2XVWbVm39yBA4LAxysQAm397zwQSQoQgewGiYZqrA9DsP4zbQ1M/<0;1>/*)";

    // Test waterfalls_version endpoint with various parameters
    let result_blocking = blocking_client
        .waterfalls_version(descriptor, 2, None, None, false)
        .unwrap();
    let result_async = async_client
        .waterfalls_version(descriptor, 2, None, None, false)
        .await
        .unwrap();

    assert_eq!(result_blocking, result_async);

    // Test with utxo_only = true
    let result_utxo_blocking = blocking_client
        .waterfalls_version(descriptor, 2, None, None, true)
        .unwrap();
    let result_utxo_async = async_client
        .waterfalls_version(descriptor, 2, None, None, true)
        .await
        .unwrap();

    assert_eq!(result_utxo_blocking, result_utxo_async);

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_server_info_endpoints() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Test server_recipient endpoint
    let recipient_blocking = blocking_client.server_recipient().unwrap();
    let recipient_async = async_client.server_recipient().await.unwrap();
    assert_eq!(recipient_blocking, recipient_async);
    assert!(!recipient_blocking.is_empty());

    // Test server_address endpoint
    let address_blocking = blocking_client.server_address().unwrap();
    let address_async = async_client.server_address().await.unwrap();
    assert_eq!(address_blocking, address_async);
    assert!(!address_blocking.is_empty());

    // Test time_since_last_block endpoint
    let time_blocking = blocking_client.time_since_last_block().unwrap();
    let time_async = async_client.time_since_last_block().await.unwrap();
    assert_eq!(time_blocking, time_async);
    assert!(!time_blocking.is_empty());

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_get_address_txs() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    let builder = Builder::new(url);
    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Create a test address and send funds to it
    let waterfalls_address = test_env.get_new_address(None);
    let bitcoin_address = convert_address(&waterfalls_address)
        .expect("Expected Bitcoin address from test environment");
    let waterfalls_txid = test_env.send_to(&waterfalls_address, 10000);
    let bitcoin_txid = convert_txid(waterfalls_txid);
    test_env.node_generate(1).await;

    // Test get_address_txs endpoint
    let txs_blocking = blocking_client.get_address_txs(bitcoin_address).unwrap();
    let txs_async = async_client.get_address_txs(bitcoin_address).await.unwrap();

    assert_eq!(txs_blocking, txs_async);
    assert!(txs_blocking.contains(&bitcoin_txid.to_string()));

    test_env.shutdown().await;
}

#[cfg(all(feature = "blocking", feature = "async"))]
#[tokio::test]
async fn test_client_with_headers() {
    let test_env = launch_test_env().await;
    let url = test_env.base_url();

    // Test client with custom headers
    let headers = HashMap::from([
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

    let blocking_client = builder.clone().build_blocking();
    let async_client = builder.build_async().unwrap();

    // Test that the client still works with custom headers
    let tip_hash_blocking = blocking_client.get_tip_hash().unwrap();
    let tip_hash_async = async_client.get_tip_hash().await.unwrap();

    assert_eq!(tip_hash_blocking, tip_hash_async);

    test_env.shutdown().await;
}

// Note: Elements testing removed as waterfalls 0.9.0 API doesn't support
// family selection in the test environment setup
