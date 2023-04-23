use crate::admin_api::AdminAPIState;
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use std::{collections::HashSet, sync::Arc};

use super::types::*;

pub fn router(state: Arc<AdminAPIState>) -> Router {
    Router::new().route("/", get(get_all)).with_state(state)
}

async fn get_all(
    State(state): State<Arc<AdminAPIState>>,
) -> Result<(StatusCode, Json<Vec<DeviceStatusInfo>>), (StatusCode, Json<ErrorResponse>)> {
    let online_devices = state
        .request_service
        .get_online_device_list()
        .await
        .map_err(err_to_response)?;
    let offline_devices: HashSet<String> = state
        .authenticator
        .read()
        .await
        .get_registered_devices()
        .await
        .into_iter()
        .filter(|id| {
            online_devices
                .iter()
                .find(|(online_id, _)| id == online_id)
                .is_none()
        })
        .collect();

    let devices: Vec<DeviceStatusInfo> =
        online_devices
            .into_iter()
            .fold(vec![], |mut acc, (id, ip)| {
                acc.push(DeviceStatusInfo {
                    id,
                    status: DeviceOnlineStatus::Online,
                    ip: Some(ip),
                });
                acc
            });
    let devices = offline_devices.into_iter().fold(devices, |mut acc, id| {
        acc.push(DeviceStatusInfo {
            id,
            status: DeviceOnlineStatus::Offline,
            ip: None,
        });
        acc
    });

    Ok((StatusCode::OK, Json(devices)))
}

fn err_to_response<E>(_err: E) -> (StatusCode, Json<ErrorResponse>) {
    // TODO: make response more informative
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal Server Error".into(),
        }),
    )
}
