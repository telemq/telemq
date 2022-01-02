use crate::session_state::SessionConnectedState;
use log::{error, info};
use mqtt_packets::v_3_1_1::ControlPacket;
use serde_json::{from_reader, to_vec};
use std::{collections::HashMap, fmt::Debug, fs::File, io, io::Write, path::Path};
use tokio::sync::RwLock;

type ClientId = String;
type InnerData = HashMap<ClientId, SessionConnectedState>;

/// Session state store where TeleMQ stores all sessions which have `clean_session: false`.
/// For the whole TeleMQ lifetime it keeps states in memory by default. If `commit` is ever called
/// `SessionStateStore` writes its inner data to file `./session_state_store.json`. In current
/// TeleMQ implementation `commit` is called just once -- during TeleMQ graceful shut down.
/// When `SessionStateStore` is being instantiated it tries to recover a state from
/// `./session_state_store.json`. If file is not found or <b>in case of any other error an empty
/// `SessionStateStore` will be created.</b>
#[derive(Debug)]
pub struct SessionStateStore {
  /// We have locks per state, so two different states can be read/modified simultanously.
  pub states: HashMap<ClientId, RwLock<SessionConnectedState>>,
}

impl SessionStateStore {
  const DATA_FILE_PATH: &'static str = "./session_state_store.json";

  pub fn new() -> SessionStateStore {
    match File::open(Path::new(Self::DATA_FILE_PATH)) {
      // try to restore an in-memory store from ./session_state_store.json
      Ok(store_data_reader) => match from_reader(store_data_reader) {
        Ok(inner_data) => Self::from_inner_data(inner_data),
        Err(err) => {
          error!(
              "[Session State Store]: to parse data from file {}. {:?}. Continue using an empty store.",
              Self::DATA_FILE_PATH, err
            );
          return SessionStateStore {
            states: HashMap::new(),
          };
        }
      },
      Err(_) => {
        error!(
          "[Session State Store]: unable to find data file {}. Continue using an empty store.",
          Self::DATA_FILE_PATH
        );
        return SessionStateStore {
          states: HashMap::new(),
        };
      }
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

  pub async fn commit(&self) -> io::Result<()> {
    let mut new_inner_data = File::open(Self::DATA_FILE_PATH)?;
    let _ = new_inner_data.set_len(0);
    new_inner_data.write_all(&to_vec(&self.as_inner_data().await).map_err(|_| {
      io::Error::new(
        io::ErrorKind::InvalidData,
        "Unable to serialize to an inner data",
      )
    })?)?;
    new_inner_data.sync_all()?;

    Ok(())
  }

  fn from_inner_data(inner_data: InnerData) -> SessionStateStore {
    let mut states = HashMap::new();

    for (client_id, state) in inner_data {
      states.insert(client_id, RwLock::new(state));
    }

    info!("[Session State Store]: recovered from a local file");

    SessionStateStore { states }
  }

  pub async fn as_inner_data(&self) -> InnerData {
    let mut inner_data = HashMap::with_capacity(self.states.len());

    for (client_id, state_lock) in &self.states {
      inner_data.insert(client_id.clone(), state_lock.read().await.clone());
    }

    inner_data
  }
}
