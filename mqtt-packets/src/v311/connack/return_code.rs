use bytes::{BufMut, BytesMut};

/// Connect acknowledge return code.
#[derive(Debug, PartialEq, Clone)]
pub enum ReturnCode {
    /// Connection Accepted
    Accepted,

    /// Connection Refused, unacceptable protocol version.
    /// The Server does not support the level of the MQTT protocol requested by the Client
    UnacceptableProtocol,

    /// Connection Refused, identifier rejected.
    /// The Client identifier is correct UTF-8 but not allowed by the Server.
    IdRejected,

    /// Connection Refused, Server unavailable.
    /// The Network Connection has been made but the MQTT service is unavailable.
    Unavailable,

    /// Connection Refused, bad user name or password.
    /// The data in the user name or password is malformed.
    BadUsernameOrPassword,

    /// Connection Refused, not authorized. The Client is not authorized to connect.
    NotAuthorized,
}

impl ReturnCode {
    const ACCEPTED_BYTE: u8 = 0x00;
    const UNACCEPTABLE_PROTOCOL_BYTE: u8 = 0x01;
    const ID_REJECTED_BYTE: u8 = 0x02;
    const UNAVAILABLE_BYTE: u8 = 0x03;
    const BAD_USERNAME_OR_PASSWORD_BYTE: u8 = 0x04;
    const NOT_AUTHORIZED_BYTE: u8 = 0x05;

    const BYTE_LEN: usize = 1;
}

/// `ReturnCode` Tokio codec.
pub struct ReturnCodeCodec;

impl ReturnCodeCodec {
    pub fn new() -> Self {
        ReturnCodeCodec {}
    }

    pub fn encode(&mut self, item: &ReturnCode, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        let byte = match item {
            &ReturnCode::Accepted => ReturnCode::ACCEPTED_BYTE,
            &ReturnCode::UnacceptableProtocol => ReturnCode::UNACCEPTABLE_PROTOCOL_BYTE,
            &ReturnCode::IdRejected => ReturnCode::ID_REJECTED_BYTE,
            &ReturnCode::Unavailable => ReturnCode::UNAVAILABLE_BYTE,
            &ReturnCode::BadUsernameOrPassword => ReturnCode::BAD_USERNAME_OR_PASSWORD_BYTE,
            &ReturnCode::NotAuthorized => ReturnCode::NOT_AUTHORIZED_BYTE,
        };
        Ok(dst.put_u8(byte))
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<ReturnCode>, std::io::Error> {
        if src.len() < ReturnCode::BYTE_LEN {
            return Ok(None);
        }
        let bytes = src.split_to(ReturnCode::BYTE_LEN);
        if bytes.len() == ReturnCode::BYTE_LEN {
            let code = match bytes[0] {
                ReturnCode::ACCEPTED_BYTE => ReturnCode::Accepted,
                ReturnCode::UNACCEPTABLE_PROTOCOL_BYTE => ReturnCode::UnacceptableProtocol,
                ReturnCode::ID_REJECTED_BYTE => ReturnCode::IdRejected,
                ReturnCode::UNAVAILABLE_BYTE => ReturnCode::Unavailable,
                ReturnCode::BAD_USERNAME_OR_PASSWORD_BYTE => ReturnCode::BadUsernameOrPassword,
                ReturnCode::NOT_AUTHORIZED_BYTE => ReturnCode::NotAuthorized,
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Cannot decode return code - unexpected byte",
                    ));
                }
            };
            Ok(Some(code))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_code_decode() {
        // decode accepted return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(ReturnCode::ACCEPTED_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .unwrap(),
                ReturnCode::Accepted
            );
        }

        // decode unacceptable protocol return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(ReturnCode::UNACCEPTABLE_PROTOCOL_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .unwrap(),
                ReturnCode::UnacceptableProtocol
            );
        }

        // decode id rejected return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(ReturnCode::ID_REJECTED_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .unwrap(),
                ReturnCode::IdRejected
            );
        }

        // decode server unavailable return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(ReturnCode::UNAVAILABLE_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .unwrap(),
                ReturnCode::Unavailable
            );
        }

        // decode bad username or password return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(ReturnCode::BAD_USERNAME_OR_PASSWORD_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .unwrap(),
                ReturnCode::BadUsernameOrPassword
            );
        }

        // decode not authorized return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(ReturnCode::NOT_AUTHORIZED_BYTE);
            assert_eq!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .unwrap(),
                ReturnCode::NotAuthorized
            );
        }

        // decode unexpected return code
        {
            let mut codec = ReturnCodeCodec {};
            let mut buf = BytesMut::new();
            assert!(
                codec
                    .decode(&mut buf)
                    .expect("Should decode ReturnCode without errors")
                    .is_none(),
                "Should keep waiting for ReturnCode bytes"
            );
            buf.put_u8(100);
            assert!(
                codec.decode(&mut buf).is_err(),
                "Should return an error on unexpected byte"
            );
        }
    }

    #[test]
    fn test_return_code_encode() {
        let mut codec = ReturnCodeCodec {};
        {
            let mut buf = BytesMut::new();
            codec
                .encode(&ReturnCode::Accepted, &mut buf)
                .expect("Should encode Accepted ReturnCode without errors");
            assert_eq!(buf, vec![ReturnCode::ACCEPTED_BYTE]);
        }

        {
            let mut buf = BytesMut::new();
            codec
                .encode(&ReturnCode::UnacceptableProtocol, &mut buf)
                .expect("Should encode UnacceptableProtocol ReturnCode without errors");
            assert_eq!(buf, vec![ReturnCode::UNACCEPTABLE_PROTOCOL_BYTE]);
        }

        {
            let mut buf = BytesMut::new();
            codec
                .encode(&ReturnCode::IdRejected, &mut buf)
                .expect("Should encode IdRejected ReturnCode without errors");
            assert_eq!(buf, vec![ReturnCode::ID_REJECTED_BYTE]);
        }

        {
            let mut buf = BytesMut::new();
            codec
                .encode(&ReturnCode::Unavailable, &mut buf)
                .expect("Should encode Unavailable ReturnCode without errors");
            assert_eq!(buf, vec![ReturnCode::UNAVAILABLE_BYTE]);
        }

        {
            let mut buf = BytesMut::new();
            codec
                .encode(&ReturnCode::BadUsernameOrPassword, &mut buf)
                .expect("Should encode BadUsernameOrPassword ReturnCode without errors");
            assert_eq!(buf, vec![ReturnCode::BAD_USERNAME_OR_PASSWORD_BYTE]);
        }

        {
            let mut buf = BytesMut::new();
            codec
                .encode(&ReturnCode::NotAuthorized, &mut buf)
                .expect("Should encode NotAuthorized ReturnCode without errors");
            assert_eq!(buf, vec![ReturnCode::NOT_AUTHORIZED_BYTE]);
        }
    }
}
