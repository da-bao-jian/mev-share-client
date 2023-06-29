use mev_share_rs::{
	EventClient,
	sse::{Event, EventStream},
};
use jsonrpsee::http_client;
use crate::signer_middleware::{FlashbotsSigner, FlashbotsSignerLayer};
use log::info;
use tower::ServiceBuilder;
use std::sync::Arc;
use ethers::{
	types::{Chain, H256},
	signers::Signer
};
use tracing_subscriber::{
	EnvFilter,
	fmt, 
	prelude::*,
};
use crate::types::{
	SupportedNetworks,
	MatchMakerNetwork
};

type FlashbotsSignerClient<S> = http_client::HttpClient<FlashbotsSigner<S, http_client::transport::HttpBackend>>;

pub struct MatchmakerClient<'a, S> {
	signer_client: FlashbotsSignerClient<S>,
	network: MatchMakerNetwork<'a>,
	event_client: EventStream<Event>
}

impl<'a, S> MatchmakerClient<'a, S> 
where S: Signer + Clone + Send + Sync + 'static
{
	async fn new(self, auth_signer: S, network: MatchMakerNetwork<'a>, event_client: EventClient) -> MatchmakerClient<'a, S> {

		let signing_middleware = FlashbotsSignerLayer::new(Arc::new(auth_signer));

		let service_builder = ServiceBuilder::new().layer(signing_middleware);

		let http_client = http_client::HttpClientBuilder::default()
			.set_middleware(service_builder)
			.build(network.api_url.clone())
			.unwrap();

		tracing_subscriber::registry().with(fmt::layer()).with(EnvFilter::from_default_env()).init();

		let event_client = event_client.events(self.network.api_url.clone()).await.unwrap();
		info!("Connected to Flashbots Matchmaker at {}", self.network.api_url);

		Self {
			signer_client: http_client,
			network,
			event_client
		}
	}

	/// Connect to Flashbots Mainnet Matchmaker
	pub async fn use_ethereum_mainnet(mut self, auth_signer: S) -> MatchmakerClient<'a, S> {
		let supported_networks = SupportedNetworks::new();
		self.network = supported_networks.get_network(Chain::Mainnet as u64).unwrap();
		let event_client = EventClient::default();
		let network = self.network.clone();
		self.new(auth_signer, network, event_client).await
	}

	/// Connect to Flashbots Goerli Matchmaker
	pub async fn use_ethereum_goerli(mut self, auth_signer: S) -> MatchmakerClient<'a, S> {
		let supported_networks = SupportedNetworks::new();
		self.network = supported_networks.get_network(Chain::Goerli as u64).unwrap();
		let event_client = EventClient::default();
		let network = self.network.clone();
		self.new(auth_signer, network, event_client).await
	}

	/// Connect to Flashbots Matchmaker given a chain id
	pub async fn from_network(mut self, auth_signer: S, chain_id: u64) -> MatchmakerClient<'a, S> {
		let supported_networks = SupportedNetworks::new();
		if !supported_networks.is_supported(chain_id) {
			panic!("Chain ID {} is not supported", chain_id);
		}
		self.network = supported_networks.get_network(chain_id).unwrap();
		let event_client = EventClient::default();
		let network = self.network.clone();
		self.new(auth_signer, network, event_client).await
	}
}