mod cli;
mod devices;
mod error;
mod get_config;

fn main() {
    let mut cmd = cli::make_command();
    let args = cmd.clone().get_matches();

    match args.subcommand() {
        Some(("device", device_args)) => devices::exec_command(device_args),
        _ => {
            println!("{}", cmd.render_help());
        }
    }
}
