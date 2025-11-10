use clap::{Arg, ArgMatches, Command};

pub fn parse_args() -> ArgMatches {
    Command::new("TeleMQ - MQTT broker")
        .arg(
            Arg::new("CONFIG_FILE")
                .short('c')
                .long("config")
                .help("TeleMQ configuration file")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("TCP_PORT")
                .help("TCP port TeleMQ will start listening on.")
                .index(1),
        )
        .get_matches()
}
