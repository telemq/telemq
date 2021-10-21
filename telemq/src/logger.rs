use crate::config::{TeleMQServerConfig, TeleMQServerConfigSrc};
use log::LevelFilter;
use log4rs::{
    append::{
        console::{ConsoleAppender, Target as ConsoleAppenderTarget},
        file::FileAppender,
    },
    config::{Appender, Config, Logger, Root},
    init_config,
};

pub fn init_logger(server_config: &TeleMQServerConfig) {
    let config_builder = Config::builder();
    let level_filter = match server_config.log_level.as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        level => {
            panic!("Unsupported logging level {}", level);
        }
    };

    let config = if server_config.log_dest == TeleMQServerConfigSrc::LOG_DEST_STDOUT {
        config_builder
            .appender(
                Appender::builder().build(
                    "stdout",
                    Box::new(
                        ConsoleAppender::builder()
                            .target(ConsoleAppenderTarget::Stdout)
                            .build(),
                    ),
                ),
            )
            .logger(Logger::builder().build("stdout", level_filter))
            .build(Root::builder().appender("stdout").build(level_filter))
            .unwrap()
    } else if server_config.log_dest == TeleMQServerConfigSrc::LOG_DEST_STDERR {
        config_builder
            .appender(
                Appender::builder().build(
                    "stderr",
                    Box::new(
                        ConsoleAppender::builder()
                            .target(ConsoleAppenderTarget::Stderr)
                            .build(),
                    ),
                ),
            )
            .logger(Logger::builder().build("stderr", level_filter))
            .build(Root::builder().appender("stderr").build(level_filter))
            .unwrap()
    } else if server_config.log_dest.starts_with("file:") {
        config_builder
            .appender(
                Appender::builder().build(
                    "file",
                    Box::new(
                        FileAppender::builder()
                            .build(server_config.log_dest.trim_start_matches("file:"))
                            .expect("Unable to build a logger according to a provided config"),
                    ),
                ),
            )
            .logger(Logger::builder().build("file", level_filter))
            .build(Root::builder().appender("file").build(level_filter))
            .unwrap()
    } else {
        unreachable!();
    };

    init_config(config).unwrap();
}
