use crate::devices::{
    device_deregister::device_deregister, device_list::device_list,
    device_register::device_register, device_update::device_update, topic_add::topic_add,
    topic_remove::topic_remove, topic_update::topic_update,
};
use crate::error::ExecResult;
use clap::{arg, builder::Str, ArgMatches, Command};

use super::device_register::device_register_batch;

pub fn make_command(name: impl Into<Str>) -> Command {
    Command::new(name)
        .about("Use this command to manage devices")
        .subcommand(
            Command::new("list").alias("ls").about("List registered devices")
        )
        .subcommand(
            Command::new("register")
                .alias("add")
                .about("Register new device which can be connected to TeleMQ")
                .args(&[
                    arg!(-d --id <CLIENT_ID> "new device unique client id").required(true),
                    arg!(-u --username <USERNAME> "new device username").required(true),
                    arg!(-p --password <PASSWORD> "new device password").required(true),
                ]),
        )
        .subcommand(
          Command::new("register-batch")
              .about("Register multiple devices which can be connected to TeleMQ")
              .args(&[
                  arg!(-f --"auth-file" <AUTH_FILE> "TeleMQ Auth file.").required(true),
              ]),
      )
        .subcommand(
            Command::new("deregister")
                .alias("remove")
                .alias("rm")
                .about("Register new device which can be connected to TeleMQ")
                .args(&[
                    arg!(-d --id <CLIENT_ID> "existing device unique client id").required(true)
                ]),
        )
        .subcommand(
            Command::new("update")
                .about("Update such device information as username and/or password")
                .args(&[
                    arg!(-d --id <CLIENT_ID> "existing device unique client id").required(true),
                    arg!(-u --username <USERNAME> "device new username").required(true),
                    arg!(-p --password <PASSWORD> "device new password").required(true),
                ]),
        )
        .subcommand(
            Command::new("topics")
                .about("Manage topics and their access rules for a device")
                .subcommand(
                    Command::new("add")
                        .about("Add new topic rule for the device")
                        .args(&[
                            arg!(-d --id <CLIENT_ID> "existing device unique client id")
                                .required(true),
                            arg!(-t --topic <TOPIC> "MQTT topic. I supports #, + and {client_id} whild cards")
                                .required(true),
                            arg!(-a --access <TOPIC> "Access level which the device will have to the topic")
                                .required(true),
                        ]),
                )
                .subcommand(
                    Command::new("remove")
                        .alias("rm")
                        .about("Remove existing topic rule from a device")
                        .args(&[
                            arg!(-d --id <CLIENT_ID> "existing device unique client id").required(true),
                            arg!(-t --topic <TOPIC> "MQTT topic. I supports #, + and {client_id} whild cards").required(true),
                        ])
                )
                .subcommand(
                    Command::new("update")
                        .about("Update existing topic rule bound to a device")
                        .args(&[
                            arg!(-d --id <CLIENT_ID> "existing device unique client id")
                                .required(true),
                            arg!(-t --topic <TOPIC> "MQTT topic. I supports #, + and {client_id} whild cards")
                                .required(true),
                            arg!(-a --access <TOPIC> "Access level which the device will have to the topic")
                                .required(true),
                        ]),
                ),
        )
}

pub fn exec_command(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("list", _)) => {
            render_result("Device List:", device_list());
        }
        Some(("register", register_args)) => {
            render_result(
                "Adding new device:",
                device_register(
                    get_mandatory_arg(register_args, "id"),
                    get_mandatory_arg(register_args, "username"),
                    get_mandatory_arg(register_args, "password"),
                ),
            );
        }
        Some(("register-batch", register_bathc_args)) => {
            device_register_batch(get_mandatory_arg(register_bathc_args, "auth-file"))
        }
        Some(("deregister", deregister_args)) => {
            device_deregister(get_mandatory_arg(deregister_args, "id"))
        }
        Some(("update", update_args)) => device_update(
            get_mandatory_arg(update_args, "id"),
            get_optional_arg(update_args, "username"),
            get_optional_arg(update_args, "password"),
        ),
        Some(("topics", topics_args)) => exec_topics(topics_args),
        _ => {
            unimplemented!()
        }
    };
}

fn exec_topics(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("add", add_topic_args)) => {
            topic_add(
                get_mandatory_arg(add_topic_args, "id"),
                get_mandatory_arg(add_topic_args, "topic"),
                get_mandatory_arg(add_topic_args, "access"),
            );
        }
        Some(("remove", rm_topic_args)) | Some(("rm", rm_topic_args)) => {
            topic_remove(
                get_mandatory_arg(rm_topic_args, "id"),
                get_mandatory_arg(rm_topic_args, "topic"),
            );
        }
        Some(("update", update_topic_args)) => {
            topic_update(
                get_mandatory_arg(update_topic_args, "id"),
                get_mandatory_arg(update_topic_args, "topic"),
                get_mandatory_arg(update_topic_args, "access"),
            );
        }
        _ => {
            unimplemented!()
        }
    }
}

fn get_mandatory_arg<T>(matches: &ArgMatches, id: &str) -> T
where
    T: Clone + Send + Sync + 'static,
{
    matches
        .get_one(id)
        .cloned()
        .expect(&format!("mandatory argument \"{}\" should be Some", id))
}

fn get_optional_arg<T>(matches: &ArgMatches, id: &str) -> Option<T>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one(id).cloned()
}

fn render_result<T, H>(header: H, res: ExecResult<T>)
where
    T: core::fmt::Debug,
    H: core::fmt::Display,
{
    match res {
        Ok(ok_res) => {
            println!("{header}\n{ok_res:?}");
        }
        Err(err) => {
            println!("Error {err:?}")
        }
    }
}
