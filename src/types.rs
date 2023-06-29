//! Types used by the Flashbot Matchmaker Client 
use ethers::types::{Address, Bytes, Chain, TxHash, U256, U64};
use mev_share_rs::sse::{Event, EventTransaction, EventTransactionLog, FunctionSelector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network configuration for the supported networks
#[derive(Debug, Clone, Default)]
pub struct SupportedNetworks<'a> {
    /// The supported networks
    supported_networks: HashMap<String, MatchMakerNetwork<'a>>,
}

impl<'a> SupportedNetworks<'a> {
    /// Creates a new instance of SupportedNetworks with the predefined network configurations.
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

    /// Retrieves the configuration for the Ethereum mainnet.
    pub fn mainnet(&self) -> Option<&MatchMakerNetwork> {
        self.supported_networks.get("mainnet")
    }

    /// Retrieves the configuration for the Ethereum Goerli testnet.
    pub fn goerli(&self) -> Option<&MatchMakerNetwork> {
        self.supported_networks.get("goerli")
    }

    /// Checks if a network with the given chain ID is supported.
    pub fn is_supported(&self, chain_id: u64) -> bool {
        self.supported_networks
            .values()
            .any(|network| network.chain_id == chain_id)
    }

    /// Retrieves the network configuration for the given chain ID.
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

/// Used to specify which type of event to listen for
pub enum StreamingEventTypes {
    /// Represents a bundle event.
    Bundle,
    /// Represents a transaction event.
    Transaction,
}

impl StreamingEventTypes {
    /// Returns the string representation of the event type.
    pub fn as_str(&self) -> &'static str {
        match self {
            StreamingEventTypes::Bundle => "bundle",
            StreamingEventTypes::Transaction => "transaction",
        }
    }
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

//////////////////////// Matchmaker Event Types ////////////////////////

/// Pending transaction from the matchmaker stream
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingTransaction {
    /// Transaction or Bundle hash.
    pub hash: TxHash,
    /// Logs emitted by the transaction or bundle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<EventTransactionLog>>,
    /// Transaction recipient address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// 4-byte-function selector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_selector: Option<FunctionSelector>,
    /// Calldata of the transaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calldata: Option<Bytes>,
    /// Change in coinbase value after inserting tx/bundle, divided by gas used
    ///
    /// Can be used to determine the minimum payment to the builder to make your backrun look more
    /// profitable to builders. Please note that this only applies to builders like Flashbots who
    /// order bundles by MEV gas price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mev_gas_price: Option<U256>,
    /// Gas used by the tx/bundle, rounded up to 2 most significant digits
    ///
    /// Only implemented on Goerli
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<U256>,
}

impl PendingTransaction {
    /// Creates a new `PendingTransaction` instance.
    pub fn new(
        hash: TxHash,
        logs: Option<Vec<EventTransactionLog>>,
        to: Option<Address>,
        function_selector: Option<FunctionSelector>,
        calldata: Option<Bytes>,
        mev_gas_price: Option<U256>,
        gas_used: Option<U256>,
    ) -> Self {
        Self {
            hash,
            logs,
            to,
            function_selector,
            calldata,
            mev_gas_price,
            gas_used,
        }
    }
}

impl From<&Event> for PendingTransaction {
    /// Converts an `Event` into a `PendingTransaction`.
    fn from(event: &Event) -> Self {
        let tx = event.transactions.clone().into_iter().next();
        Self {
            hash: event.hash,
            logs: Some(event.logs.clone()),
            to: tx.as_ref().map(|tx| tx.to),
            function_selector: tx.as_ref().map(|tx| tx.function_selector.clone()),
            calldata: tx.map(|tx| tx.calldata),
            mev_gas_price: None,
            gas_used: None,
        }
    }
}

/// Pending bundle from the matchmaker stream
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingBundle {
    /// Transaction or Bundle hash.
    pub hash: TxHash,
    /// Logs emitted by the transaction or bundle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<EventTransactionLog>>,
    /// Transaction details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<EventTransaction>>,
    /// Change in coinbase value after inserting tx/bundle, divided by gas used
    ///
    /// Can be used to determine the minimum payment to the builder to make your backrun look more
    /// profitable to builders. Please note that this only applies to builders like Flashbots who
    /// order bundles by MEV gas price.
    #[serde(skip_serializing_if = "Option::is_none")]
    mev_gas_price: Option<U256>,
    /// Gas used by the tx/bundle, rounded up to 2 most significant digits
    ///
    /// Only implemented on Goerli
    #[serde(skip_serializing_if = "Option::is_none")]
    gas_used: Option<U256>,
}

impl PendingBundle {
    /// Creates a new `PendingBundle` instance.
    pub fn new(
        hash: TxHash,
        logs: Option<Vec<EventTransactionLog>>,
        transactions: Option<Vec<EventTransaction>>,
        mev_gas_price: Option<U256>,
        gas_used: Option<U256>,
    ) -> Self {
        Self {
            hash,
            logs,
            transactions,
            mev_gas_price,
            gas_used,
        }
    }
}

impl From<&Event> for PendingBundle {
    /// Converts an `Event` into a `PendingBundle`.
    fn from(event: &Event) -> Self {
        Self {
            hash: event.hash,
            logs: Some(event.logs.clone()),
            transactions: Some(event.transactions.clone()),
            mev_gas_price: None,
            gas_used: None,
        }
    }
}

/// Pending transaction or bundle from the matchmaker stream
pub enum PendingTxOrBundle {
    Tx(PendingTransaction),
    Bundle(PendingBundle),
}
