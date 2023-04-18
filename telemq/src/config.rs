use std::{
    fs::read_to_string as read_file,
    io::Error as IoError,
    net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::Path,
    str::FromStr,
    time::Duration,
};

use ipnet::IpNet;
use regex::Regex;
use serde::Deserialize;
use serde_json::{from_str as json_from_str, Error as JsonError};
use toml::{de::Error as TomlError, from_str as toml_from_str};

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
    pub cluster_id: OptString,
    pub account_id: OptString,
    pub max_connections: OptUsize,
    pub tcp_port: OptPort,
    pub tls_port: OptPort,
    pub cert_file: OptString,
    pub key_file: OptString,
    pub ws_port: OptPort,
    pub wss_port: OptPort,
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
    pub admin_api_port: OptPort,
    pub ip_whitelist: OptList<String>,
}

impl TeleMQServerConfigSrc {
    pub const LOG_DEST_STDOUT: &'static str = "stdout";
    pub const LOG_DEST_STDERR: &'static str = "stderr";
    pub const LOG_DEST_FILE_REGEX: &'static str = "^(file:)";
    pub const LOG_LEVEL: &'static [&'static str; 4] = &["error", "warn", "info", "debug"];
    const FILE_TOML_EXTENSION: &'static str = "toml";
    const FILE_JSON_EXTENSION: &'static str = "json";

    pub fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let config_file_content = read_file(&path)?;
        let config_file_extension = path.as_ref().extension().and_then(|os_str| os_str.to_str());
        let config_src: TeleMQServerConfigSrc = match config_file_extension {
            Some(Self::FILE_TOML_EXTENSION) => toml_from_str(&config_file_content)?,
            Some(Self::FILE_JSON_EXTENSION) => json_from_str(&config_file_content)?,
            _ => {
                unimplemented!();
            }
        };
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
            .and_then(|_| Self::validate_state_store_url(&config_src.session_state_store_url))
            .and_then(|_| Self::validate_broker_id(&config_src.broker_id))
            .and_then(|_| Self::validate_cluster_id(&config_src.cluster_id))
            .and_then(|_| Self::validate_account_id(&config_src.account_id))
            .and_then(|_| Self::validate_ip_whitelist(&config_src.ip_whitelist))
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
          Self::LOG_DEST_STDERR
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
        // if let Some(auth_url) = auth_endpoint {
        //     return Ok(());
        //     if auth_url.parse::<SocketAddr>().is_err() {
        //         return Err(TeleMQServerConfigError::WrongValue(format!("Invalid authentication configuration. auth_endpoint is not a valid socket addr")));
        //     }
        // }
        return Ok(());
    }

    fn validate_state_store_url(maybe_state_store_url: &OptString) -> ConfigResult<()> {
        match maybe_state_store_url {
            Some(state_store_url) => {
                if state_store_url.to_socket_addrs().is_err() {
                    return Err(TeleMQServerConfigError::WrongValue(
                        "Cannot parse session_state_store_url into a socket address".into(),
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

    fn validate_cluster_id(cluster_id: &OptString) -> ConfigResult<()> {
        if cluster_id.is_some() {
            return Ok(());
        }

        return Err(TeleMQServerConfigError::WrongValue(
            "cluster_id is a mandatory field".into(),
        ));
    }

    fn validate_account_id(account_id: &OptString) -> ConfigResult<()> {
        if account_id.is_some() {
            return Ok(());
        }

        return Err(TeleMQServerConfigError::WrongValue(
            "account_id is a mandatory field".into(),
        ));
    }

    fn validate_ip_whitelist(ip_whitelist: &Option<Vec<String>>) -> ConfigResult<()> {
        if ip_whitelist.is_none() {
            return Ok(());
        }

        for ref ip_net in ip_whitelist.as_ref().unwrap() {
            if IpNet::from_str(ip_net).is_err() {
                return Err(TeleMQServerConfigError::WrongValue(
                    "ip_whitelist contains a value which cannot be parsed into IP network address"
                        .into(),
                ));
            }
        }

        return Ok(());
    }
}

#[derive(Debug)]
pub struct TeleMQServerConfig {
    pub max_connections: usize,
    // TCP listener
    pub tcp_addr: SocketAddr,
    // TLS Listener
    pub tls_addr: OptSocketAddr,
    pub cert_file: OptString,
    pub key_file: OptString,
    // Websocket listener
    pub ws_addr: OptSocketAddr,
    pub wss_addr: OptSocketAddr,
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
    pub auth_endpoint: OptString,
    pub auth_file: OptString,
    pub sys_topics_update_interval: Duration,
    pub session_state_store_url: OptSocketAddr,
    pub admin_api: OptSocketAddr,
    pub ip_whitelist: Option<Vec<IpNet>>,
}

impl From<TeleMQServerConfigSrc> for TeleMQServerConfig {
    fn from(src: TeleMQServerConfigSrc) -> Self {
        let with_tls = src.cert_file.is_some();
        TeleMQServerConfig {
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
            ws_addr: src.ws_port.map(local_listener),
            wss_addr: src.wss_port.map(local_listener),
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
            auth_endpoint: src.auth_endpoint,
            auth_file: src.auth_file,
            sys_topics_update_interval: src
                .sys_topics_update_interval
                .map(|secs| {
                    if secs == 0 {
                        Duration::ZERO
                    } else {
                        Duration::from_secs(secs)
                    }
                })
                .unwrap_or_else(|| Duration::from_secs(Self::DEFAULT_SYS_TOPICS_UPDATE_INTERVAL)),
            session_state_store_url: src.session_state_store_url.map(|url| url.parse().unwrap()),
            admin_api: src.admin_api_port.map(|port| local_listener(port)),
            ip_whitelist: src.ip_whitelist.map(|ip_net_strs| {
                ip_net_strs
                    .iter()
                    .map(|ip_net_str| ip_net_str.parse().unwrap())
                    .collect()
            }),
        }
    }
}

impl Default for TeleMQServerConfig {
    fn default() -> Self {
        TeleMQServerConfig {
            max_connections: Self::DEFAULT_MAX_CONNECTIONS,
            tcp_addr: local_listener(Self::DEFAULT_TCP_PORT),
            tls_addr: None,
            cert_file: None,
            key_file: None,
            ws_addr: None,
            wss_addr: None,
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
            session_state_store_url: None,
            admin_api: None,
            ip_whitelist: None,
        }
    }
}

impl TeleMQServerConfig {
    pub const DEFAULT_MAX_CONNECTIONS: usize = 10_000;
    pub const DEFAULT_TCP_PORT: u16 = 1883;
    pub const DEFAULT_TLS_PORT: u16 = 8883;
    pub const DEFAULT_ACTIVITY_CHECK_INTERVAL: u64 = 120;
    pub const DEFAULT_BACKUP_INTERVAL: u64 = 30;
    pub const DEFAULT_KEEP_ALIVE: u64 = 120;
    pub const DEFAULT_LOG: &'static str = "stdout";
    pub const DEFAULT_LOG_LEVEL: &'static str = "info";
    pub const DEFAULT_ANONYMOUS_ALLOWED: bool = true;
    pub const DEFAULT_SYS_TOPICS_UPDATE_INTERVAL: u64 = 30;

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

impl From<JsonError> for TeleMQServerConfigError {
    fn from(err: JsonError) -> Self {
        TeleMQServerConfigError::ConfigFile(format!("{:?}", err))
    }
}

type ConfigResult<T> = Result<T, TeleMQServerConfigError>;
