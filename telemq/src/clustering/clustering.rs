use serde::Deserialize;
use std::{
  convert::TryFrom,
  io::Error as IoError,
  net::{AddrParseError, SocketAddr},
};

use mqtt_packets::v_3_1_1::topic::Subscription;

#[derive(Deserialize, Debug)]
pub struct ClusteringConfigSrc {
  pub peers: Vec<String>,
  pub shared_topics: Vec<String>,
}

#[derive(Debug)]
pub struct ClusteringConfigConvertError {
  pub error: String,
}

impl From<AddrParseError> for ClusteringConfigConvertError {
  fn from(error: AddrParseError) -> Self {
    ClusteringConfigConvertError {
      error: error.to_string(),
    }
  }
}

impl From<IoError> for ClusteringConfigConvertError {
  fn from(error: IoError) -> Self {
    ClusteringConfigConvertError {
      error: error.to_string(),
    }
  }
}

#[derive(Debug, PartialEq)]
pub struct ClusteringConfig {
  pub peers: Vec<PeerConfig>,
  pub shared_topics: Vec<Subscription>,
}

impl TryFrom<ClusteringConfigSrc> for ClusteringConfig {
  type Error = ClusteringConfigConvertError;

  fn try_from(src: ClusteringConfigSrc) -> Result<Self, Self::Error> {
    let mut peers: Vec<PeerConfig> = Vec::with_capacity(src.peers.len());

    for peer in src.peers {
      peers.push(PeerConfig {
        addr: peer.parse().map_err(ClusteringConfigConvertError::from)?,
      });
    }

    let mut shared_topics = Vec::with_capacity(src.shared_topics.len());

    for t in src.shared_topics {
      shared_topics.push(Subscription::try_from(&t).map_err(ClusteringConfigConvertError::from)?);
    }

    Ok(ClusteringConfig {
      peers,
      shared_topics: shared_topics,
    })
  }
}

#[derive(Debug, PartialEq)]
pub struct PeerConfig {
  pub addr: SocketAddr,
}

#[cfg(test)]
mod clustering_config_src_test {
  use super::*;
  use toml::from_str;

  #[test]
  fn should_deserialize_correct_config() {
    let toml_src = r###"
      peers = [
        "127.0.0.1:1886",
        "127.0.0.2:1886"
      ]
      shared_topics = []
     "###;

    let res: ClusteringConfigSrc =
      from_str(toml_src).expect("should parse clustering config without errors");

    assert_eq!(res.peers, vec!["127.0.0.1:1886", "127.0.0.2:1886"]);
  }
}

#[cfg(test)]
mod clustering_config_test {
  use super::*;
  use std::convert::TryFrom;
  use toml::from_str;

  #[test]
  fn should_deserialize_correct_config_without_shared_topics() {
    let toml_src = r###"
      peers = [
        "127.0.0.1:1886",
        "127.0.0.2:1886"
      ]
      shared_topics = []
     "###;

    let res: ClusteringConfigSrc =
      from_str(toml_src).expect("should parse clustering config without errors");

    assert_eq!(res.peers, vec!["127.0.0.1:1886", "127.0.0.2:1886"]);
    assert_eq!(res.shared_topics, vec![] as Vec<String>);
  }

  #[test]
  fn should_deserialize_correct_config_with_shared_topics() {
    let toml_src = r###"
      peers = []
      shared_topics = ["some/topic"]
     "###;

    let res: ClusteringConfigSrc =
      from_str(toml_src).expect("should parse clustering config without errors");

    assert_eq!(res.shared_topics, vec!["some/topic".to_string()]);
  }

  #[test]
  fn should_convert_from_src_without_shared_topics_without_errors() {
    let src = ClusteringConfigSrc {
      peers: vec!["127.0.0.1:1886".to_owned(), "127.0.0.2:1886".to_owned()],
      shared_topics: vec![],
    };

    let config = ClusteringConfig::try_from(src).expect("should convert without errors");
    assert_eq!(
      config,
      ClusteringConfig {
        peers: vec![PeerConfig {
          addr: "127.0.0.1:1886".parse().unwrap()
        }],
        shared_topics: vec![]
      }
    );
  }

  #[test]
  fn should_convert_from_src_with_shared_topics_without_errors() {
    let src = ClusteringConfigSrc {
      peers: vec![],
      shared_topics: vec!["some/topic".to_string()],
    };

    let config = ClusteringConfig::try_from(src).expect("should convert without errors");
    assert_eq!(
      config,
      ClusteringConfig {
        peers: vec![],
        shared_topics: vec![Subscription {
          original: "some/topic".to_string(),
          path: vec!["some".to_string(), "topic".to_string()]
        }]
      }
    );
  }
}
