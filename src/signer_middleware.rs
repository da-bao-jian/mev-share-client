// Credit to FrankieIsLost
// Copied from https://github.com/paradigmxyz/artemis/blob/main/crates/clients/matchmaker/src/flashbots_signer.rs

//! A layer responsible for implementing flashbots-style authentication
//! by signing the request body with a private key and adding the signature
//! to the request headers.

use std::{
    sync::Arc,
    task::{Context, Poll},
};

use ethers::{signers::Signer, types::H256, utils::keccak256};
use futures_util::future::BoxFuture;

use http::{header::HeaderValue, HeaderName, Request};
use hyper::Body;

use tower::{Layer, Service};

/// Layer that applies [`FlashbotsSigner`] which adds a request header with a signed payload.
#[derive(Clone)]
pub(crate) struct FlashbotsSignerLayer<S> {
    signer: Arc<S>,
}

impl<S> FlashbotsSignerLayer<S> {
    pub(crate) fn new(signer: Arc<S>) -> Self {
        FlashbotsSignerLayer { signer }
    }
}

impl<S: Clone, I> Layer<I> for FlashbotsSignerLayer<S> {
    type Service = FlashbotsSigner<S, I>;

    fn layer(&self, inner: I) -> Self::Service {
        FlashbotsSigner {
            signer: self.signer.clone(),
            inner,
        }
    }
}

/// Middleware that adds a request header with a signed payload.
/// For more info, see https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#authentication
#[derive(Clone)]
pub struct FlashbotsSigner<S, I> {
    signer: Arc<S>,
    inner: I,
}

impl<S, I> Service<Request<Body>> for FlashbotsSigner<S, I>
where
    I: Service<Request<Body>> + Clone + Send + 'static,
    I::Future: Send,
    S: Signer + Clone + Send + 'static,
{
    type Response = I::Response;
    type Error = I::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        // wait for service to be ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let signer = self.signer.clone();

        let (mut parts, body) = request.into_parts();

        Box::pin(async move {
            let body_bytes = hyper::body::to_bytes(body).await.unwrap();

            // sign request body and insert header
            let signature = signer
                .sign_message(format!("0x{:x}", H256::from(keccak256(body_bytes.clone()))))
                .await
                .unwrap();

            let header_name = HeaderName::from_static("x-flashbots-signature");
            let header_val =
                HeaderValue::from_str(&format!("{:?}:0x{}", signer.address(), signature)).unwrap();
            parts.headers.insert(header_name, header_val);

            let request = Request::from_parts(parts, Body::from(body_bytes.clone()));

            inner.call(request).await
        })
    }
}
