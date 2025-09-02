pub use bitcoin::consensus::{deserialize, serialize};
pub use bitcoin::hex::FromHex;
use bitcoin::Weight;
pub use bitcoin::{
    transaction, Amount, BlockHash, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Response from the waterfalls endpoint
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct WaterfallResponse {
    pub txs_seen: BTreeMap<String, Vec<Vec<TxSeen>>>,
    pub page: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<BlockHash>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip_meta: Option<BlockMeta>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Ord, PartialOrd)]
pub struct BlockMeta {
    /// The block hash
    pub b: BlockHash,

    /// The block timestamp
    pub t: u32,

    /// The block height
    pub h: u32,
}

/// A transaction seen in the blockchain for a specific script
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct TxSeen {
    pub txid: Txid,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<BlockHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_timestamp: Option<u32>,
    #[serde(skip_serializing_if = "V::is_undefined", default)]
    pub v: V,
}

/// Enum representing whether a transaction was seen in a vout or vin
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum V {
    #[default]
    Undefined,
    Vin(u32),
    Vout(u32),
}

impl V {
    pub fn is_undefined(&self) -> bool {
        matches!(self, V::Undefined)
    }

    pub fn raw(&self) -> i32 {
        match self {
            V::Undefined => 0,
            V::Vout(n) => *n as i32,
            V::Vin(n) => -(*n as i32) - 1,
        }
    }

    pub fn from_raw(raw: i32) -> Self {
        match raw {
            0 => V::Undefined,
            x if x > 0 => V::Vout(x as u32),
            x => V::Vin((-x - 1) as u32),
        }
    }
}

impl Serialize for V {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.raw())
    }
}

impl<'de> Deserialize<'de> for V {
    fn deserialize<D>(deserializer: D) -> Result<V, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = i32::deserialize(deserializer)?;
        Ok(V::from_raw(raw))
    }
}

impl WaterfallResponse {
    pub fn is_empty(&self) -> bool {
        self.txs_seen
            .iter()
            .flat_map(|(_, v)| v.iter())
            .all(|a| a.is_empty())
    }
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PrevOut {
    pub value: u64,
    pub scriptpubkey: ScriptBuf,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Vin {
    pub txid: Txid,
    pub vout: u32,
    // None if coinbase
    pub prevout: Option<PrevOut>,
    pub scriptsig: ScriptBuf,
    #[serde(deserialize_with = "deserialize_witness", default)]
    pub witness: Vec<Vec<u8>>,
    pub sequence: u32,
    pub is_coinbase: bool,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Vout {
    pub value: u64,
    pub scriptpubkey: ScriptBuf,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TxStatus {
    pub confirmed: bool,
    pub block_height: Option<u32>,
    pub block_hash: Option<BlockHash>,
    pub block_time: Option<u64>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub block_height: u32,
    pub merkle: Vec<Txid>,
    pub pos: usize,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct OutputStatus {
    pub spent: bool,
    pub txid: Option<Txid>,
    pub vin: Option<u64>,
    pub status: Option<TxStatus>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockStatus {
    pub in_best_chain: bool,
    pub height: Option<u32>,
    pub next_best: Option<BlockHash>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Tx {
    pub txid: Txid,
    pub version: i32,
    pub locktime: u32,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
    /// Transaction size in raw bytes (NOT virtual bytes).
    pub size: usize,
    /// Transaction weight units.
    pub weight: u64,
    pub status: TxStatus,
    pub fee: u64,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockTime {
    pub timestamp: u64,
    pub height: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct BlockSummary {
    pub id: BlockHash,
    #[serde(flatten)]
    pub time: BlockTime,
    /// Hash of the previous block, will be `None` for the genesis block.
    pub previousblockhash: Option<bitcoin::BlockHash>,
    pub merkle_root: bitcoin::hash_types::TxMerkleNode,
}

/// Address statistics, includes the address, and the utxo information for the address.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AddressStats {
    /// The address.
    pub address: String,
    /// The summary of transactions for this address, already on chain.
    pub chain_stats: AddressTxsSummary,
    /// The summary of transactions for this address, currently in the mempool.
    pub mempool_stats: AddressTxsSummary,
}

/// Contains a summary of the transactions for an address.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
pub struct AddressTxsSummary {
    /// The number of funded transaction outputs.
    pub funded_txo_count: u32,
    /// The sum of the funded transaction outputs, in satoshis.
    pub funded_txo_sum: u64,
    /// The number of spent transaction outputs.
    pub spent_txo_count: u32,
    /// The sum of the spent transaction outputs, in satoshis.
    pub spent_txo_sum: u64,
    /// The total number of transactions.
    pub tx_count: u32,
}

impl Tx {
    pub fn to_tx(&self) -> Transaction {
        Transaction {
            version: transaction::Version::non_standard(self.version),
            lock_time: bitcoin::absolute::LockTime::from_consensus(self.locktime),
            input: self
                .vin
                .iter()
                .cloned()
                .map(|vin| TxIn {
                    previous_output: OutPoint {
                        txid: vin.txid,
                        vout: vin.vout,
                    },
                    script_sig: vin.scriptsig,
                    sequence: bitcoin::Sequence(vin.sequence),
                    witness: Witness::from_slice(&vin.witness),
                })
                .collect(),
            output: self
                .vout
                .iter()
                .cloned()
                .map(|vout| TxOut {
                    value: Amount::from_sat(vout.value),
                    script_pubkey: vout.scriptpubkey,
                })
                .collect(),
        }
    }

    pub fn confirmation_time(&self) -> Option<BlockTime> {
        match self.status {
            TxStatus {
                confirmed: true,
                block_height: Some(height),
                block_time: Some(timestamp),
                ..
            } => Some(BlockTime { timestamp, height }),
            _ => None,
        }
    }

    pub fn previous_outputs(&self) -> Vec<Option<TxOut>> {
        self.vin
            .iter()
            .cloned()
            .map(|vin| {
                vin.prevout.map(|po| TxOut {
                    script_pubkey: po.scriptpubkey,
                    value: Amount::from_sat(po.value),
                })
            })
            .collect()
    }

    pub fn weight(&self) -> Weight {
        Weight::from_wu(self.weight)
    }

    pub fn fee(&self) -> Amount {
        Amount::from_sat(self.fee)
    }
}

fn deserialize_witness<'de, D>(d: D) -> Result<Vec<Vec<u8>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let list = Vec::<String>::deserialize(d)?;
    list.into_iter()
        .map(|hex_str| Vec::<u8>::from_hex(&hex_str))
        .collect::<Result<Vec<Vec<u8>>, _>>()
        .map_err(serde::de::Error::custom)
}
