extern crate bytes;
extern crate clap;
extern crate crypto;
extern crate futures;
extern crate ipnet;
extern crate log;
extern crate log4rs;
#[cfg(test)]
extern crate maplit;
extern crate mqtt_packets;
extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate signal_hook;
extern crate signal_hook_tokio;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_stream;
extern crate tokio_util;
extern crate toml;
extern crate warp;

mod args;
mod authenticator;
mod config;
mod connection;
mod connection_provider;
mod control;
mod logger;
mod net_connection;
mod server;
mod server_error;
mod session_error;
mod session_state;
mod session_state_store;
mod stats;
mod subscription_tree;
mod tls_listener;
mod transaction;
mod websocket_listener;

use args::parse_args;
use config::TeleMQServerConfig;
use logger::init_logger;
use server::Server;
use std::{
    error::Error,
    io::{stderr, Write},
    process::exit,
};

#[tokio::main(worker_threads = 25)]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args();
    let mut config = match args.value_of("CONFIG_FILE") {
        Some(config_file) => match TeleMQServerConfig::from_file(config_file) {
            Ok(c) => c,
            Err(err) => {
                stderr().write(format!("{:?}\n", err).as_bytes()).unwrap();
                exit(1);
            }
        },
        None => TeleMQServerConfig::default(),
    };

    if let Some(arg_port_str) = args.value_of("TCP_PORT") {
        match arg_port_str.parse::<u16>() {
            Ok(port) => config.tcp_addr.set_port(port),
            Err(_) => {
                stderr()
                    .write("Unable to parse TCP provided as a first argument.\n".as_bytes())
                    .unwrap();
                exit(1);
            }
        }
    }

    init_logger(&config);

    if let Some(server) = Server::new(config).await {
        server.start().await?;
    };

    Ok(())
}
