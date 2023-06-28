use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub(crate) struct TestBundle {
    genesis_alloc: HashMap<String, GenesisAlloc>,
    header: Header,
    tests: Vec<Test>,
}

#[derive(Debug, Deserialize)]
struct GenesisAlloc {
    balance: String,
    code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Header {
    parent_hash: String,
    sha3_uncles: String,
    miner: String,
    state_root: String,
    transactions_root: String,
    receipts_root: String,
    logs_bloom: String,
    number: String,
    gas_limit: String,
    gas_used: String,
    timestamp: String,
    extra_data: String,
    mix_hash: String,
    nonce: String,
    base_fee_per_gas: String,
    withdrawals_root: Option<String>,
    hash: String,
}

#[derive(Debug, Deserialize)]
struct Test {
    name: String,
    bundle: Bundle,
    should_fail: bool,
}

#[derive(Debug, Deserialize)]
struct Bundle {
    version: String,
    inclusion: Inclusion,
    body: Vec<Body>,
    validity: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Inclusion {
    block: String,
}

#[derive(Debug, Deserialize)]
struct Body {
    tx: String,
}
