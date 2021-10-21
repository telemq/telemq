use std::{
    fs::read as read_file,
    io::Error as IoError,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use mqtt_packets::v_3_1_1::topic::Topic;

use regex::Regex;
use serde::Deserialize;
use toml::{de::Error as TomlError, from_slice};

type OptPort = Option<u16>;
type OptUsize = Option<usize>;
type OptString = Option<String>;
type OptDuration = Option<u64>;
type OptBool = Option<bool>;
type OptSocketAddr = Option<SocketAddr>;
type OptList<T> = Option<Vec<T>>;

#[derive(Deserialize)]
pub struct TeleMQServerConfigSrc {
    pub broker_id: OptString,
    pub max_connections: OptUsize,
    pub tcp_port: OptPort,
    pub tls_port: OptPort,
    pub cert_file: OptString,
    pub key_file: OptString,
    pub websockets_allowed: OptBool,
    pub web_port: OptPort,
    pub activity_check_interval: OptDuration,
    pub backup_interval: OptDuration,
    pub keep_alive: OptDuration,
    /// stdout, stderr, file:telemq.log
    pub log_dest: OptString,
    pub log_level: OptString,
    pub max_packet_size: OptUsize,
    pub max_subs_per_client: OptUsize,
    pub max_storage_duration: OptDuration,
    pub anonymous_allowed: OptBool,
    pub auth_endpoint: OptString,
    pub auth_file: OptString,
    pub sys_topics_update_interval: OptDuration,
    pub session_state_store_url: OptString,
    pub bridge_in_port: OptPort,
    pub bridge_out: OptList<BridgeOutConfigSrc>,
    pub admin_api_port: OptPort,
}

impl TeleMQServerConfigSrc {
    pub const LOG_DEST_STDOUT: &'static str = "stdout";
    pub const LOG_DEST_STDERR: &'static str = "stdout";
    pub const LOG_DEST_FILE_REGEX: &'static str = "^(file:)";
    pub const LOG_LEVEL: &'static [&'static str; 4] = &["error", "warn", "info", "debug"];

    pub fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let config_file_content = read_file(path)?;
        let config_src: TeleMQServerConfigSrc = from_slice(config_file_content.as_slice())?;
        Self::validate(&config_src)?;
        Ok(config_src)
    }

    fn validate(config_src: &TeleMQServerConfigSrc) -> ConfigResult<()> {
        Self::validate_log_dest(&config_src.log_dest)
            .and_then(|_| Self::validate_log_level(&config_src.log_level))
            .and_then(|_| {
                Self::validate_auth(
                    &config_src.anonymous_allowed,
                    &config_src.auth_file,
                    &config_src.auth_endpoint,
                )
            })
            .and_then(|_| Self::validate_bridge_out(&config_src.bridge_out))
            .and_then(|_| Self::validate_broker_id(&config_src.broker_id))
    }

    fn validate_log_dest(maybe_log_dest: &OptString) -> ConfigResult<()> {
        match maybe_log_dest {
            Some(log_dest) => {
                if log_dest == Self::LOG_DEST_STDOUT
                    || log_dest == Self::LOG_DEST_STDERR
                    || Regex::new(Self::LOG_DEST_FILE_REGEX)
                        .unwrap()
                        .is_match_at(log_dest, 0)
                {
                    return Ok(());
                }

                return Err(TeleMQServerConfigError::WrongValue(format!(
          "Unsupported log destination \"{}\".\nSupported values: \"{}\", \"{}\", \"file:<path>\"",
          log_dest,
          Self::LOG_DEST_STDOUT,
          log_dest == Self::LOG_DEST_STDERR
        )));
            }
            None => Ok(()),
        }
    }

    fn validate_log_level(maybe_log_level: &OptString) -> ConfigResult<()> {
        match maybe_log_level {
            Some(log_level) => {
                if Self::LOG_LEVEL
                    .iter()
                    .any(|supported| supported == log_level)
                {
                    return Ok(());
                }
                return Err(TeleMQServerConfigError::WrongValue(format!(
                    "Unsupported log level {}.\nSupported values: {:?}",
                    log_level,
                    Self::LOG_LEVEL
                )));
            }
            None => Ok(()),
        }
    }

    fn validate_auth(
        anonymous_allowed: &OptBool,
        auth_file: &OptString,
        auth_endpoint: &OptString,
    ) -> ConfigResult<()> {
        if !anonymous_allowed.unwrap_or(true) && auth_file.is_none() && auth_endpoint.is_none() {
            return Err(TeleMQServerConfigError::WrongValue(format!("Invalid authentication configuration. Allow anonymous usage, or provide authentication endpoint or provide authentication file.")));
        }
        if let Some(auth_url) = auth_endpoint {
            if auth_url.parse::<SocketAddr>().is_err() {
                return Err(TeleMQServerConfigError::WrongValue(format!("Invalid authentication configuration. auth_endpoint is not a valid socket addr")));
            }
        }
        return Ok(());
    }

    fn validate_bridge_out(maybe_bridge_out: &OptList<BridgeOutConfigSrc>) -> ConfigResult<()> {
        match maybe_bridge_out {
            Some(bridge_out) => {
                if bridge_out.iter().any(|b| b.host.to_socket_addrs().is_err()) {
                    return Err(TeleMQServerConfigError::WrongValue(
                        "Cannot parse on of bride_in_addrs".into(),
                    ));
                }

                return Ok(());
            }
            None => Ok(()),
        }
    }

    fn validate_broker_id(broker_id: &OptString) -> ConfigResult<()> {
        if broker_id.is_some() {
            return Ok(());
        }

        return Err(TeleMQServerConfigError::WrongValue(
            "broker_id is a mandatory field".into(),
        ));
    }
}

#[derive(Debug)]
pub struct TeleMQServerConfig {
    pub broker_id: String,
    pub max_connections: usize,
    // TCP listener
    pub tcp_addr: SocketAddr,
    // TLS Listener
    pub tls_addr: OptSocketAddr,
    pub cert_file: OptString,
    pub key_file: OptString,
    // Websocket listener
    pub web_addr: OptSocketAddr,
    pub activity_check_interval: Duration,
    pub backup_interval: Duration,
    pub keep_alive: Duration,
    /// stdout, stderr, file:telemq.log
    pub log_dest: String,
    pub log_level: String,
    // if None => unlimited
    pub max_packet_size: OptUsize,
    // if None => unlimited
    pub max_subs_per_client: OptUsize,
    // if None => unlimited
    pub max_storage_duration: OptDuration,
    pub anonymous_allowed: bool,
    pub auth_endpoint: OptSocketAddr,
    pub auth_file: OptString,
    pub sys_topics_update_interval: Duration,
    pub session_state_store_url: String,
    pub bridge_in_addr: OptSocketAddr,
    pub bridge_out: OptList<BridgeOutConfig>,
    pub admin_api: OptSocketAddr,
}

impl From<TeleMQServerConfigSrc> for TeleMQServerConfig {
    fn from(src: TeleMQServerConfigSrc) -> Self {
        let with_tls = src.cert_file.is_some();
        TeleMQServerConfig {
            broker_id: src
                .broker_id
                .unwrap_or_else(|| Self::DEFAULT_BROKER_ID.to_string()),
            max_connections: src.max_connections.unwrap_or(Self::DEFAULT_MAX_CONNECTIONS),
            tcp_addr: local_listener(src.tcp_port.unwrap_or(Self::DEFAULT_TCP_PORT)),
            tls_addr: if with_tls {
                Some(local_listener(
                    src.tls_port.unwrap_or(Self::DEFAULT_TLS_PORT),
                ))
            } else {
                None
            },
            cert_file: src.cert_file,
            key_file: src.key_file,
            web_addr: if src
                .websockets_allowed
                .unwrap_or(Self::DEFAULT_WEBSOCKETS_ALLOWED)
            {
                Some(local_listener(
                    src.web_port.unwrap_or(Self::DEFAULT_TLS_PORT),
                ))
            } else {
                None
            },
            activity_check_interval: Duration::from_secs(
                src.activity_check_interval
                    .unwrap_or(Self::DEFAULT_ACTIVITY_CHECK_INTERVAL),
            ),
            backup_interval: Duration::from_secs(
                src.backup_interval.unwrap_or(Self::DEFAULT_BACKUP_INTERVAL),
            ),
            keep_alive: Duration::from_secs(src.keep_alive.unwrap_or(Self::DEFAULT_KEEP_ALIVE)),
            log_dest: src
                .log_dest
                .unwrap_or_else(|| Self::DEFAULT_LOG.to_string()),
            log_level: src
                .log_level
                .unwrap_or_else(|| Self::DEFAULT_LOG_LEVEL.to_string()),
            max_packet_size: src.max_packet_size,
            max_subs_per_client: src.max_subs_per_client,
            max_storage_duration: src.max_storage_duration,
            anonymous_allowed: match src.anonymous_allowed {
                Some(v) => v,
                None => {
                    src.auth_endpoint.is_none()
                        && src.auth_file.is_none()
                        && Self::DEFAULT_ANONYMOUS_ALLOWED
                }
            },
            auth_endpoint: src.auth_endpoint.map(|url| url.parse().unwrap()),
            auth_file: src.auth_file,
            sys_topics_update_interval: Duration::from_secs(
                src.sys_topics_update_interval
                    .unwrap_or(Self::DEFAULT_SYS_TOPICS_UPDATE_INTERVAL),
            ),
            session_state_store_url: src
                .session_state_store_url
                .unwrap_or(Self::DEFAULT_SESSION_STATE_STORE_URL.to_string()),
            bridge_in_addr: src
                .bridge_in_port
                .map(|bridge_in_port| local_listener(bridge_in_port)),
            bridge_out: src.bridge_out.map(|bb| {
                // validated value, safe to unwrap
                bb.iter()
                    .map(|b| BridgeOutConfig {
                        host: b.host.split(":").next().unwrap().to_string(),
                        addr: b.host.to_socket_addrs().unwrap().next().unwrap(),
                        topics: b.topics.iter().map(Topic::make_from_string).collect(),
                    })
                    .collect()
            }),
            admin_api: src.admin_api_port.map(|port| local_listener(port)),
        }
    }
}

impl Default for TeleMQServerConfig {
    fn default() -> Self {
        TeleMQServerConfig {
            broker_id: Self::DEFAULT_BROKER_ID.to_string(),
            max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            tcp_addr: local_listener(Self::DEFAULT_TCP_PORT),
            tls_addr: None,
            cert_file: None,
            key_file: None,
            web_addr: None,
            activity_check_interval: Duration::from_secs(Self::DEFAULT_ACTIVITY_CHECK_INTERVAL),
            backup_interval: Duration::from_secs(Self::DEFAULT_BACKUP_INTERVAL),
            keep_alive: Duration::from_secs(Self::DEFAULT_KEEP_ALIVE),
            log_dest: Self::DEFAULT_LOG.to_string(),
            log_level: Self::DEFAULT_LOG_LEVEL.to_string(),
            // Infinite
            max_packet_size: None,
            // Infinite
            max_subs_per_client: None,
            // Infinite
            max_storage_duration: None,
            anonymous_allowed: Self::DEFAULT_ANONYMOUS_ALLOWED,
            auth_endpoint: None,
            auth_file: None,
            sys_topics_update_interval: Duration::from_secs(
                Self::DEFAULT_SYS_TOPICS_UPDATE_INTERVAL,
            ),
            session_state_store_url: Self::DEFAULT_SESSION_STATE_STORE_URL.to_string(),
            bridge_in_addr: None,
            bridge_out: None,
            admin_api: None,
        }
    }
}

impl TeleMQServerConfig {
    pub const DEFAULT_BROKER_ID: &'static str = "<undefined>";
    pub const DEFAULT_MAX_CONNECTIONS: usize = 10_000;
    pub const DEFAULT_TCP_PORT: u16 = 1883;
    pub const DEFAULT_TLS_PORT: u16 = 8883;
    pub const DEFAULT_WEBSOCKETS_ALLOWED: bool = false;
    pub const DEFAULT_ACTIVITY_CHECK_INTERVAL: u64 = 120;
    pub const DEFAULT_BACKUP_INTERVAL: u64 = 30;
    pub const DEFAULT_KEEP_ALIVE: u64 = 120;
    pub const DEFAULT_LOG: &'static str = "stdout";
    pub const DEFAULT_LOG_LEVEL: &'static str = "info";
    pub const DEFAULT_ANONYMOUS_ALLOWED: bool = true;
    pub const DEFAULT_SYS_TOPICS_UPDATE_INTERVAL: u64 = 30;
    pub const DEFAULT_SESSION_STATE_STORE_URL: &'static str = "http://localhost:8086";

    pub fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        TeleMQServerConfigSrc::from_file(path).map(From::from)
    }
}

fn local_listener(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
}

#[derive(Debug)]
pub enum TeleMQServerConfigError {
    ConfigFile(String),
    WrongValue(String),
}

impl From<IoError> for TeleMQServerConfigError {
    fn from(err: IoError) -> Self {
        TeleMQServerConfigError::ConfigFile(format!("{:?}", err))
    }
}

impl From<TomlError> for TeleMQServerConfigError {
    fn from(err: TomlError) -> Self {
        TeleMQServerConfigError::ConfigFile(format!("{:?}", err))
    }
}

type ConfigResult<T> = Result<T, TeleMQServerConfigError>;

#[derive(Deserialize)]
pub struct BridgeOutConfigSrc {
    host: String,
    topics: Vec<String>,
}

#[derive(Debug)]
pub struct BridgeOutConfig {
    pub host: String,
    pub addr: SocketAddr,
    pub topics: Vec<Topic>,
}
