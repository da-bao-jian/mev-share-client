//! A Flashbots client for interacting with the Flashbots Matchmaker service
//! based on https://github.com/flashbots/matchmaker-ts
use crate::signer_middleware::{FlashbotsSigner, FlashbotsSignerLayer};
use crate::types::{
    Bundle, MatchMakerNetwork, PendingBundle, PendingTransaction, PendingTxOrBundle,
    SendBundleResponse, StreamingEventTypes, SupportedNetworks,
};
use anyhow::Result;
use ethers::{signers::Signer, types::Chain};
use futures_util::StreamExt;
use jsonrpsee::{core::client::ClientT, http_client};
use log::{error, info};
use mev_share_rs::{sse::Event, EventClient};
use parking_lot::Mutex;
use std::sync::Arc;
use tower::ServiceBuilder;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

type FlashbotsSignerClient<S> =
    http_client::HttpClient<FlashbotsSigner<S, http_client::transport::HttpBackend>>;

/// Struct representing a client for interacting with the Flashbots Matchmaker service
#[allow(dead_code)]
pub struct MatchmakerClient<'a, S> {
    signer_client: FlashbotsSignerClient<S>,
    network: MatchMakerNetwork<'a>,
    event_client: EventClient,
}

impl<'a, S> MatchmakerClient<'a, S>
where
    S: Signer + Clone + 'static,
{
    /// Constructs a new `MatchmakerClient` with the provided parameters
    ///
    /// * `auth_signer` - A Signer used for signing tx
    /// * `network` - The network that the client will connect to
    /// * `event_client` - A client for handling incoming events
    #[allow(clippy::wrong_self_convention)]
    fn new(
        self,
        auth_signer: S,
        network: MatchMakerNetwork<'a>,
        event_client: EventClient,
    ) -> MatchmakerClient<'a, S> {
        let signing_middleware = FlashbotsSignerLayer::new(Arc::new(auth_signer));

        let service_builder = ServiceBuilder::new().layer(signing_middleware);

        let http_client = http_client::HttpClientBuilder::default()
            .set_middleware(service_builder)
            .build(network.api_url)
            .unwrap();

        Self {
            signer_client: http_client,
            network,
            event_client,
        }
    }

    /// Connect to Flashbots Mainnet Matchmaker
    ///
    /// * `auth_signer` - A Signer used for signing tx
    pub fn use_ethereum_mainnet(mut self, auth_signer: S) -> MatchmakerClient<'a, S> {
        let supported_networks = SupportedNetworks::new();
        self.network = supported_networks
            .get_network(Chain::Mainnet as u64)
            .unwrap();
        let event_client = EventClient::default();
        let network = self.network.clone();
        self.new(auth_signer, network, event_client)
    }

    /// Connect to Flashbots Goerli Matchmaker
    ///     
    /// * `auth_signer` - A Signer used for signing tx
    pub fn use_ethereum_goerli(mut self, auth_signer: S) -> MatchmakerClient<'a, S> {
        let supported_networks = SupportedNetworks::new();
        self.network = supported_networks
            .get_network(Chain::Goerli as u64)
            .unwrap();
        let event_client = EventClient::default();
        let network = self.network.clone();
        self.new(auth_signer, network, event_client)
    }

    /// Connect to supported networks by specifying a network with a `chain_id`
    ///     
    /// * `auth_signer` - A Signer used for signing tx
    /// * `chain_id` - ID of the chain to connect to
    pub async fn from_network(mut self, auth_signer: S, chain_id: u64) -> MatchmakerClient<'a, S> {
        let supported_networks = SupportedNetworks::new();
        if !supported_networks.is_supported(chain_id) {
            panic!("Chain ID {} is not supported", chain_id);
        }
        self.network = supported_networks.get_network(chain_id).unwrap();
        let event_client = EventClient::default();
        let network = self.network.clone();
        self.new(auth_signer, network, event_client)
    }

    /// Registers the provided callback to be called when a new MEV-Share transaction is received.
    ///
    /// * `event` - The event received from the event stream.
    /// * `callback` - Async function to process pending tx.
    fn on_transaction<F>(&self, event: &Event, callback: F)
    where
        F: FnOnce(PendingTxOrBundle),
    {
        let tx = PendingTransaction::from(event);
        callback(PendingTxOrBundle::Tx(tx));
    }

    /// Registers the provided callback to be called when a new MEV-Share bundle is received.
    ///
    /// * `event` - The event received from the event stream.
    /// * `callback` - Async function to process pending bundle.
    fn on_bundle<F>(&self, event: &Event, callback: F)
    where
        F: FnOnce(PendingTxOrBundle),
    {
        let bundle = PendingBundle::from(event);
        callback(PendingTxOrBundle::Bundle(bundle));
    }

    /// Starts listening to the Matchmaker event stream and registers the given callback to be invoked when the given event type is received
    ///
    /// * `event_type` - Type of the event to listen for
    /// * `callback` - Function that will be called when a new event is received
    pub async fn on<F>(&self, event_type: StreamingEventTypes, callback: F)
    where
        F: FnMut(PendingTxOrBundle) + Send + Sync + 'static,
    {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .init();

        let mut stream = self
            .event_client
            .events(self.network.stream_url)
            .await
            .unwrap();

        info!(
            "Connected to Flashbots Matchmaker at {}",
            self.network.stream_url
        );

        let callback = Arc::new(Mutex::new(callback));
        let event_handler: Box<dyn Fn(Event) + Send + Sync> = match event_type {
            StreamingEventTypes::Bundle => {
                info!("Listening for Bundle events");
                Box::new(|pending_event: Event| {
                    self.on_bundle(&pending_event, &mut *callback.lock());
                })
            }
            StreamingEventTypes::Transaction => {
                info!("Listening for Bundle events");
                Box::new(|pending_event: Event| {
                    self.on_transaction(&pending_event, &mut *callback.lock());
                })
            }
        };

        // TODO: add Event enum to allow dynamic dispatch
        while let Some(event) = stream.next().await {
            match event {
                Ok(e) => {
                    event_handler(e);
                }
                Err(e) => {
                    error!("Error: {:?}", e);
                }
            }
        }
    }

    /// Sends a bundle to mev-share
    ///
    /// * `bundle` - Params for the bundle to be sent
    pub async fn send_bundle(&self, bundle: &Bundle) -> Result<SendBundleResponse> {
        let response = self
            .signer_client
            .request("mev_sendBundle", [bundle])
            .await?;

        Ok(response)
    }
}
