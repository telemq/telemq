mod devices;
mod request_service;

use axum::Router;
use hyper::Error as ServerError;
use request_service::RequestService;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub use request_service::{AdminApiInMessage, AdminApiOutMessage, OnlineDevices};

pub type AdminApiResponseSender = mpsc::UnboundedSender<AdminApiInMessage>;
pub type AdminApiRequestReceiver = mpsc::UnboundedReceiver<AdminApiOutMessage>;

pub fn create_inbound_channel() -> (
    AdminApiResponseSender,
    mpsc::UnboundedReceiver<AdminApiInMessage>,
) {
    mpsc::unbounded_channel()
}

pub struct AdminAPI {
    addr: SocketAddr,
    state: Arc<AdminAPIState>,
}

pub struct AdminAPIState {
    request_service: RequestService,
    authenticator: Arc<RwLock<crate::authenticator::Authenticator>>,
}

impl AdminAPI {
    pub fn new(
        addr: SocketAddr,
        response_receiver: mpsc::UnboundedReceiver<AdminApiInMessage>,
        authenticator: Arc<RwLock<crate::authenticator::Authenticator>>,
    ) -> (Self, AdminApiRequestReceiver) {
        let (tx, rx) = mpsc::unbounded_channel::<AdminApiOutMessage>();
        let state = Arc::new(AdminAPIState {
            request_service: RequestService::new(response_receiver, tx),
            authenticator,
        });

        (AdminAPI { addr, state }, rx)
    }

    pub async fn run(self) -> Result<(), ServerError> {
        let app = Router::new().nest("/devices", devices::router(self.state.clone()));

        axum::Server::bind(&self.addr)
            .serve(app.into_make_service())
            .await
    }
}
