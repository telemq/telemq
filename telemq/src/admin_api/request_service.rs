use futures::future::Future;
use log::error;
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
    task::Waker,
};
use tokio::{spawn, sync::mpsc};
use uuid::Uuid;

pub type OnlineDevices = Vec<(String, IpAddr)>;

pub enum AdminApiInMessage {
    OnlineDevicesList { req_id: Uuid, list: OnlineDevices },
}

impl AdminApiInMessage {
    fn id(&self) -> &Uuid {
        match self {
            AdminApiInMessage::OnlineDevicesList { req_id, .. } => req_id,
        }
    }
}

pub enum AdminApiOutMessage {
    OnlineDevicesList { req_id: Uuid },
}

enum Response {
    Waker(Waker),
    Response(AdminApiInMessage),
}

#[derive(Debug, PartialEq)]
pub enum RequestServiceError {
    InternalError(String),
}

type RequestsMap = Arc<RwLock<HashMap<Uuid, Response>>>;

pub struct RequestService {
    requests: RequestsMap,
    request_sender: mpsc::UnboundedSender<AdminApiOutMessage>,
}

impl RequestService {
    pub fn new(
        mut response_receiver: mpsc::UnboundedReceiver<AdminApiInMessage>,
        request_sender: mpsc::UnboundedSender<AdminApiOutMessage>,
    ) -> Self {
        let requests = Arc::new(RwLock::new(HashMap::new()));
        let requests_copy = requests.clone();

        spawn(async move {
            loop {
                match response_receiver.recv().await {
                    Some(response) => {
                        if let Ok(mut reqs_mut) = requests_copy.write() {
                            if let Some(Response::Waker(waker)) = reqs_mut.remove(response.id()) {
                                reqs_mut
                                    .insert(response.id().clone(), Response::Response(response));
                                waker.wake();
                            } else {
                                // It's rather a bug of implementation of the RequestService itself
                                error!("[Admin API Request Service] cannot find pending request with UUID {:?}", response.id());
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        RequestService {
            requests,
            // response_receiver,
            request_sender,
        }
    }

    pub async fn get_online_device_list(&self) -> Result<OnlineDevices, RequestServiceError> {
        let request_fut = GetDeviceRequest::new(&self.requests);
        if let Err(err) = self
            .request_sender
            .send(AdminApiOutMessage::OnlineDevicesList {
                req_id: request_fut.id.clone(),
            })
        {
            return Err(RequestServiceError::InternalError(err.to_string()));
        }
        request_fut.await
    }
}

pub struct GetDeviceRequest {
    id: Uuid,
    requests: RequestsMap,
}

impl GetDeviceRequest {
    fn new(requests: &RequestsMap) -> Self {
        GetDeviceRequest {
            id: Uuid::new_v4(),
            requests: requests.clone(),
        }
    }
}

impl Future for GetDeviceRequest {
    type Output = Result<OnlineDevices, RequestServiceError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if let Ok(mut reqs_mut) = self.requests.write() {
            match reqs_mut.remove(&self.id) {
                Some(response_entry) => match response_entry {
                    Response::Response(response) => match response {
                        AdminApiInMessage::OnlineDevicesList { list, .. } => {
                            std::task::Poll::Ready(Ok(list))
                        } // _ => std::task::Poll::Ready(Err(RequestServiceError::TypesDontMatch)),
                    },
                    Response::Waker(_) => std::task::Poll::Pending,
                },
                None => {
                    reqs_mut.insert(self.id.clone(), Response::Waker(cx.waker().clone()));
                    std::task::Poll::Pending
                }
            }
        } else {
            std::task::Poll::Ready(Err(RequestServiceError::InternalError(
                "Cannot acquire write lock on Request Service".into(),
            )))
        }
    }
}
