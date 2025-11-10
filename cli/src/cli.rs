use clap::Command;

use crate::devices;

pub fn make_command() -> Command {
    Command::new("TeleMQ CLI")
        .name("TeleMQ CLI")
        .about("CLI for managing TeleMQ clusters and instances")
        .subcommand(devices::make_command("device"))
}
