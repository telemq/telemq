use crate::session_state::SessionConnectedState;
use mqtt_packets::v_3_1_1::ControlPacket;
use std::{collections::HashMap, io};
use tokio::sync::RwLock;

type ClientId = String;

#[derive(Debug)]
pub struct SessionStateStore {
  /// We have locks per state, so two different states can be read/modified simultanously.
  states: HashMap<ClientId, RwLock<SessionConnectedState>>,
}

impl SessionStateStore {
  pub fn new() -> SessionStateStore {
    SessionStateStore {
      states: HashMap::new(),
    }
  }

  pub async fn save_state(&mut self, state: SessionConnectedState) -> io::Result<()> {
    let client_id = state.client_id.clone();
    self.states.insert(client_id, RwLock::new(state));

    Ok(())
  }

  pub async fn take_state(
    &mut self,
    client_id: &ClientId,
  ) -> io::Result<Option<SessionConnectedState>> {
    Ok(
      self
        .states
        .remove(client_id)
        .map(|maybe_state_rw_lock| maybe_state_rw_lock.into_inner()),
    )
  }

  pub async fn new_publish(&self, client_id: &ClientId, packet: ControlPacket) -> io::Result<()> {
    if let Some(session) = self.states.get(client_id) {
      session
        .write()
        .await
        .messages_pending_transmition
        .push_back(packet.clone());
    }

    Ok(())
  }
}
