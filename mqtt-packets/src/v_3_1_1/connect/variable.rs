use bytes::{BufMut, BytesMut};
use regex::Regex;

use super::{
    connect_flags::{ConnectFlags, ConnectFlagsCodec},
    keep_alive::{KeepAlive, KeepAliveCodec},
    protocol_level::{ProtocolLevel, ProtocolLevelCodec},
    protocol_name::{ProtocolName, ProtocolNameCodec},
};
use crate::v_3_1_1::topic::Topic;
use crate::v_3_1_1::utils::codec as utils_codec;

/// Variable header + payload specific for Connect Control Packet.
#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
    // variable header
    pub protocol_name: ProtocolName,
    pub protocol_level: ProtocolLevel,
    pub connect_flags: ConnectFlags,
    pub keep_alive: KeepAlive,
    // variable payload
    pub client_identifier: String,
    pub will_topic: Option<Topic>,
    pub will_message: Option<Vec<u8>>,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub struct VariableCodec {
    protocol_name_codec: ProtocolNameCodec,
    protocol_level_codec: ProtocolLevelCodec,
    connect_flags_codec: ConnectFlagsCodec,
    keep_alive_codec: KeepAliveCodec,
}

impl VariableCodec {
    pub fn new() -> Self {
        VariableCodec {
            protocol_name_codec: ProtocolNameCodec::new(),
            protocol_level_codec: ProtocolLevelCodec::new(),
            connect_flags_codec: ConnectFlagsCodec::new(),
            keep_alive_codec: KeepAliveCodec::new(),
        }
    }

    pub fn encode(&mut self, item: &Variable, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        self.protocol_name_codec.encode(&item.protocol_name, dst)?;
        self.protocol_level_codec
            .encode(&item.protocol_level, dst)?;
        self.connect_flags_codec.encode(&item.connect_flags, dst)?;
        self.keep_alive_codec.encode(&item.keep_alive, dst)?;
        {
            Self::validate_client_id(&item.client_identifier)?;
            let encoded_id = item.client_identifier.as_bytes();
            dst.put_u16(encoded_id.len() as u16);
            dst.put(encoded_id);
        }

        self.encode_will_topic(&item, dst)?;
        self.encode_username(&item, dst)?;
        self.encode_password(&item, dst)?;

        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Variable>, std::io::Error> {
        // Variable header
        // FIXME: rewrite codecs to not return Option since
        // because a variable will be decoded after getting remaining length
        // number of bytes it's assumed that
        // all required data is already in a buffer
        let protocol_name = self.protocol_name_codec.decode(src)?.unwrap();
        let protocol_level = self.protocol_level_codec.decode(src)?.unwrap();
        let connect_flags = self.connect_flags_codec.decode(src)?.unwrap();
        let keep_alive = self.keep_alive_codec.decode(src)?.unwrap();
        // Payload
        let client_identifier = utils_codec::decode_optional_string(src).unwrap();
        let will_topic = if connect_flags.has_will_flag() {
            match utils_codec::decode_optional_string(src) {
                Some(topic) => Some(Topic::try_from(topic)?),
                None => None,
            }
        } else {
            None
        };
        let will_message = if connect_flags.has_will_flag() {
            utils_codec::decode_optional_bytes(src)
        } else {
            None
        };
        let username = if connect_flags.has_username() {
            utils_codec::decode_optional_string(src)
        } else {
            None
        };
        let password = if connect_flags.has_password() {
            utils_codec::decode_optional_string(src)
        } else {
            None
        };

        let client_identifier_validity = Self::validate_client_id(&client_identifier);

        if client_identifier_validity.is_err() {
            Err(client_identifier_validity.err().unwrap())
        } else {
            Ok(Some(Variable {
                protocol_name,
                protocol_level,
                connect_flags,
                keep_alive,
                client_identifier,
                will_topic,
                will_message,
                username,
                password,
            }))
        }
    }

    fn validate_client_id(client_id: &String) -> ::std::io::Result<()> {
        lazy_static! {
            static ref CLIENT_ID_REGEX: Regex = Regex::new(r"^[0-9A-z]{0, 23}$").unwrap();
        }

        if !CLIENT_ID_REGEX.is_match(client_id) {
            return Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Client ID contains non numerical, non alphabetical symbols or has\
             length more than 23",
            ));
        }

        Ok(())
    }

    /// It encodes will topic and check consistency between Control Packet flags and its payload.
    /// It returns an error if, for instance, `item.will_topic` is some, but `flags` does not
    /// have will flag activated.
    fn encode_will_topic(
        &mut self,
        item: &Variable,
        dst: &mut BytesMut,
    ) -> Result<(), ::std::io::Error> {
        if item.will_topic.is_some()
            && item.will_message.is_some()
            && item.connect_flags.has_will_flag()
        {
            utils_codec::encode_optional_string(
                &item
                    .will_topic
                    .as_ref()
                    .map(|topic_op| topic_op.original.clone()),
                dst,
            );
            utils_codec::encode_optional_bytes(&item.will_message, dst);
            Ok(())
        } else if item.will_topic.is_none()
            && item.will_message.is_none()
            && !item.connect_flags.has_will_flag()
        {
            Ok(())
        } else {
            Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Control Package: inconsistency between payload (will topic and/or will message)\
               and connect flags",
            ))
        }
    }

    /// It encodes will topic and check consistency between Control Packet flags and its payload.
    /// It returns an error if, for instance, `item.username` is some, but `flags` does not
    /// have username activated.
    fn encode_username(
        &mut self,
        item: &Variable,
        dst: &mut BytesMut,
    ) -> Result<(), ::std::io::Error> {
        if item.username.is_some() && item.connect_flags.has_username() {
            utils_codec::encode_optional_string(&item.username, dst);
            Ok(())
        } else if item.username.is_none() && !item.connect_flags.has_username() {
            Ok(())
        } else {
            Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Control Package: inconsistency between payload (username) and connect flags",
            ))
        }
    }

    /// It encodes will topic and check consistency between Control Packet flags and its payload.
    /// It returns an error if, for instance, `item.password` is some, but `flags` does not
    /// have password activated.
    fn encode_password(
        &mut self,
        item: &Variable,
        dst: &mut BytesMut,
    ) -> Result<(), ::std::io::Error> {
        if item.password.is_some() && item.connect_flags.has_password() {
            utils_codec::encode_optional_string(&item.password, dst);
            Ok(())
        } else if item.password.is_none() && !item.connect_flags.has_password() {
            Ok(())
        } else {
            Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Control Package: inconsistency between payload (password) and connect flags",
            ))
        }
    }
}
