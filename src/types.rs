use ethers::types::{Address, Chain, Bytes, TxHash, H256, U256, U64};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{collections::HashMap, array::TryFromSliceError, fmt::LowerHex, ops::Deref};


/// Network configuration for the supported networks
pub struct SupportedNetworks<'a> {
    /// The supported networks
    supported_networks: HashMap<String, MatchMakerNetwork<'a>>,
}

impl<'a> SupportedNetworks<'a> {
    pub fn new() -> Self {
        let mut networks = HashMap::new();
        networks.insert(
            "mainnet".to_string(),
            MatchMakerNetwork {
                name: "mainnet",
                chain_id: Chain::Mainnet.into(),
                stream_url: "https://mev-share.flashbots.net",
                api_url: "https://relay.flashbots.net",
            },
        );
        networks.insert(
            "goerli".to_string(),
            MatchMakerNetwork {
                name: "goerli",
                chain_id: Chain::Goerli.into(),
                stream_url: "https://mev-share-goerli.flashbots.net",
                api_url: "https://relay-goerli.flashbots.net",
            },
        );

        SupportedNetworks {
            supported_networks: networks,
        }
    }

    pub fn mainnet(&self) -> Option<&MatchMakerNetwork> {
        self.supported_networks.get("mainnet")
    }

    pub fn goerli(&self) -> Option<&MatchMakerNetwork> {
        self.supported_networks.get("goerli")
    }

    pub fn is_supported(&self, chain_id: u64) -> bool {
        self.supported_networks
            .values()
            .any(|network| network.chain_id == chain_id)
    }

    pub fn get_network(&self, chain: u64) -> Option<MatchMakerNetwork<'a>> {
        self.supported_networks
            .values()
            .find(|network| network.chain_id == chain)
            .cloned()
    }
}

/// Configuration used to connect to the Matchmaker
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MatchMakerNetwork<'a> {
	/// Chain ID of network
	pub chain_id: u64,
	/// Lowercase name of network. e.g. "mainnet"
	pub name: &'a str,
	/// The URL of the Matchmaker API
	pub stream_url: &'a str,
	/// Matchmaker bundle & transaction API URL
	pub api_url: &'a str,
}

/// Smart bundle spec version
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub enum ProtocolVersion {
    /// The 0.1 version of the API.
    #[default]
    #[serde(rename = "v0.1")]
    V1,
}

/// Conditions for the bundle to be considered for inclusion in a block, evaluated _before_ the bundle is placed in a block.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionParams {
    /// Target block number in which to include the bundle.
    pub block: U64,
    /// Maximum block height in which the bundle can be included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_block: Option<U64>,
}

/// Transactions that make up the bundle. `hash` refers to a transaction hash from the Matchmaker event stream.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BundleTx {
    /// The hash of the transaction we are trying to backrun.
    TxHash {
        /// Tx hash.
        hash: TxHash,
    },
    /// A new signed transaction.
    #[serde(rename_all = "camelCase")]
    Tx {
        /// Bytes of the signed transaction.
        tx: Bytes,
        /// If true, the transaction can revert without the bundle being considered invalid.
        can_revert: bool,
    },
}

/// Bundle privacy parameters
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyParams {
    /// Data fields from bundle transactions to be shared with searchers on MEV-Share
    #[serde(skip_serializing_if = "Option::is_none")]
    hints: Option<HintPreference>,
    /// Builders that are allowed to receive this bundle. See [mev-share spec](https://github.com/flashbots/mev-share/blob/main/builders/registration.json) for supported builders.
    builders: Vec<String>,
}

/// Conditions for receiving refunds
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Refund {
    /// Index of entry in `body` to which the refund percentage applies
    body_idx: usize,
    /// Minimum refund percentage required for this bundle to be eligible for use by another searcher
    percent: u32,
}

/// Specifies how refund should be paid if bundle is used by another searcher
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RefundConfig {
    /// The address that receives this portion of the refund
    address: String,
    /// Percentage of refund to be paid to `address`. Set this to `100` unless splitting refunds between multiple recipients
    percent: u32,
}

/// Conditions for bundle to be considered for inclusion in a block, evaluated _after_ the bundle is placed in the block
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidityParams {
    /// Conditions for receiving refunds
    #[serde(skip_serializing_if = "Option::is_none")]
    refund: Option<Vec<Refund>>,
    /// Specifies how refund should be paid if bundle is used by another searcher
    #[serde(skip_serializing_if = "Option::is_none")]
    refund_config: Option<Vec<RefundConfig>>,
}

/// Parameters sent to mev_sendBundle
pub struct Bundle {
    /// Smart bundle spec version
    pub version: ProtocolVersion,
    /// Conditions for the bundle to be considered for inclusion in a block, evaluated _before_ the bundle is placed in a block
    pub inclusion: InclusionParams,
    /// Transactions that make up the bundle. `hash` refers to a transaction hash from the Matchmaker event stream
    pub body: Vec<BundleTx>,
    /// Conditions for bundle to be considered for inclusion in a block, evaluated _after_ the bundle is placed in the block.
    pub validity: Option<ValidityParams>,
    /// Bundle privacy parameters
    pub privacy: Option<PrivacyParams>,
}

/// Bundle details
#[derive(Debug, Serialize, Deserialize)]
pub struct SendBundleResult {
    /// Bundle hash
    pub bundle_hash: String,
}

/// Response received from matchmaker API
#[derive(Debug, Serialize, Deserialize)]
pub struct SendBundleResponse {
    /// Bundle hash
    pub bundle_hash: String,
}

/// Decodes a raw sendBundle response
impl SendBundleResult {
    pub fn from_response(response: &SendBundleResponse) -> Self {
        SendBundleResult {
            bundle_hash: response.bundle_hash.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Parameters accepted by the [send_transaction] function
pub struct TransactionOptions {
    /// Hints define what data about a transaction is shared with searchers
    #[serde(skip_serializing_if = "Option::is_none")]
    hints: Option<HintPreference>,
    /// Maximum block number for the transaction to be included in
    #[serde(skip_serializing_if = "Option::is_none")]
    max_block_number: Option<U64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    builders: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HintPreference {
    /// Share the calldata of the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    calldata: Option<bool>,
    /// Share the contract address of the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    contract_address: Option<bool>,
    /// Share the 4byte function selector of the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    function_selector: Option<bool>,
    /// Share the logs emitted by the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    logs: Option<bool>,
    /// Share tx hashes of transactions in bundle
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_hash: Option<bool>,
}

//////////////////////// Event History Types ////////////////////////

/// Data about the event history endpoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct EventHistoryInfo {
    pub count: u64,
    pub min_block: u64,
    pub max_block: u64,
    pub min_timestamp: u64,
    pub max_timestamp: u64,
    pub max_limit: u64,
}

/// SSE event of the `history` endpoint
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventHistory {
    /// The block number of the event's block.
    pub block: u64,
    /// The timestamp when the event was emitted.
    pub timestamp: u64,
    /// Hint for the historic block.
    pub hint: Hint,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct Hint {
    #[serde(with = "null_sequence")]
    pub txs: Vec<EventTransaction>,
    pub hash: H256,
    #[serde(with = "null_sequence")]
    pub logs: Vec<EventTransactionLog>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<U256>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mev_gas_price: Option<U256>,
}

/// Query params for the `history` endpoint
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct EventHistoryParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_end: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_end: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

#[allow(missing_docs)]
impl EventHistoryParams {
    pub fn with_block_start(mut self, block_start: u64) -> Self {
        self.block_start = Some(block_start);
        self
    }

    pub fn with_block_end(mut self, block_end: u64) -> Self {
        self.block_end = Some(block_end);
        self
    }

    pub fn with_block_range(mut self, block_start: u64, block_end: u64) -> Self {
        self.block_start = Some(block_start);
        self.block_end = Some(block_end);
        self
    }

    pub fn with_timestamp_start(mut self, timestamp_start: u64) -> Self {
        self.timestamp_start = Some(timestamp_start);
        self
    }

    pub fn with_timestamp_end(mut self, timestamp_end: u64) -> Self {
        self.timestamp_end = Some(timestamp_end);
        self
    }

    pub fn with_timestamp_range(mut self, timestamp_start: u64, timestamp_end: u64) -> Self {
        self.timestamp_start = Some(timestamp_start);
        self.timestamp_end = Some(timestamp_end);
        self
    }

    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}

//////////////////////// Matchmaker Event Types ////////////////////////

/// API wrapper for events received by the SSE stream
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MatchMakerEvent {
    ///  Transaction or Bundle hash
    pub hash: TxHash,
    /// Event logs emitted by executing the transaction
    pub log: Vec<EventTransactionLog>,
    /// Logs emitted by the transaction or bundle
    pub transactions: Vec<EventTransaction>,
}

/// Transaction from the event
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventTransaction {
    /// Transaction recipient address.
    pub to: Address,
    /// 4-byte-function selector
    #[serde(rename = "functionSelector")]
    pub function_selector: FunctionSelector,
    /// Calldata of the transaction
    #[serde(rename = "callData")]
    pub calldata: Bytes,
}

/// A log produced by a transaction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventTransactionLog {
    /// The address of the contract that emitted the log
    pub address: Address,
    /// Topics of the log
    ///
    /// (In solidity: The first topic is the hash of the signature of the event
    /// (e.g. `Deposit(address,bytes32,uint256)`), except you declared the event
    /// with the anonymous specifier.)
    pub topics: Vec<H256>,
}

/// 4-byte-function selector
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunctionSelector(pub [u8; 4]);
impl FunctionSelector {
    fn hex_encode(&self) -> String {
        hex::encode(self.0.as_ref())
    }
}

impl Serialize for FunctionSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for FunctionSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let s = hex_str.strip_prefix("0x").unwrap_or(&hex_str);
        if s.len() != 8 {
            return Err(serde::de::Error::custom(format!(
                "Expected 4 byte function selector: {}",
                hex_str
            )));
        }

        let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
        let selector =
            FunctionSelector::try_from(bytes.as_slice()).map_err(serde::de::Error::custom)?;
        Ok(selector)
    }
}

impl AsRef<[u8]> for FunctionSelector {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl std::fmt::Debug for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FunctionSelector")
            .field(&self.hex_encode())
            .finish()
    }
}

impl std::fmt::Display for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.hex_encode())
    }
}

impl LowerHex for FunctionSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", self.hex_encode())
    }
}

impl Deref for FunctionSelector {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

impl From<[u8; 4]> for FunctionSelector {
    fn from(src: [u8; 4]) -> Self {
        Self(src)
    }
}

impl<'a> TryFrom<&'a [u8]> for FunctionSelector {
    type Error = TryFromSliceError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let sel: [u8; 4] = value.try_into()?;
        Ok(Self(sel))
    }
}
impl PartialEq<[u8; 4]> for FunctionSelector {
    fn eq(&self, other: &[u8; 4]) -> bool {
        other == &self.0
    }
}

/// Deserializes missing or null sequences as empty vectors.
mod null_sequence {
    use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

    pub(crate) fn deserialize<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: DeserializeOwned,
    {
        let s = Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default();
        Ok(s)
    }

    pub(crate) fn serialize<T, S>(val: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        if val.is_empty() {
            serializer.serialize_none()
        } else {
            val.serialize(serializer)
        }
    }
}
