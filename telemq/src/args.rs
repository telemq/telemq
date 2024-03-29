use clap::{App, Arg, ArgMatches};

pub fn parse_args() -> ArgMatches {
    App::new("TeleMQ - MQTT broker")
        .arg(
            Arg::new("CONFIG_FILE")
                .short('c')
                .long("config")
                .help("TeleMQ configuration file")
                .takes_value(true),
        )
        .arg(
            Arg::new("TCP_PORT")
                .help("TCP port TeleMQ will start listening on.")
                .index(1),
        )
        .get_matches()
}
