pub mod codec {
    use bytes::{Buf, BufMut, BytesMut};
    use std::convert::TryInto;

    /// Length of string bytes prefix which points to a length of following string.
    const BYTES_LEN_LEN: usize = 2;

    /// It encodes a given string
    pub fn encode_string(string: &str, dst: &mut BytesMut) {
        let encoded = string.as_bytes();
        dst.put_u16(encoded.len() as u16);
        dst.extend_from_slice(encoded);
    }

    /// It encodes a given optional `String` field.
    pub fn encode_optional_string<T: AsRef<str>>(prop: &Option<T>, dst: &mut BytesMut) {
        prop.as_ref().and_then(|p| {
            encode_string(p.as_ref(), dst);
            None as Option<()>
        });
    }

    /// It encodes a given optional `String` field.
    pub fn encode_optional_bytes(prop: &Option<Vec<u8>>, dst: &mut BytesMut) {
        prop.as_ref().and_then(|bytes| {
            dst.put_u16(bytes.len() as u16);
            dst.extend_from_slice(bytes);
            None as Option<()>
        });
    }

    /// It decodes an optional string. If it's able to read first two bytes which describe
    /// string's length and all the bytes which represent a string itself then it returns
    /// `Some` with `String`, otherwise `None` would be returned.
    pub fn decode_optional_string(src: &mut BytesMut) -> Option<String> {
        src.get(0..BYTES_LEN_LEN)
            .and_then(|len_bytes| len_bytes.to_vec().try_into().ok().map(u16::from_be_bytes))
            .and_then(|len| {
                let len_usize = len as usize;
                if src.len() >= len_usize + BYTES_LEN_LEN {
                    // remove first two bytes as a length of the following string
                    src.advance(BYTES_LEN_LEN);
                    let bytes = src.split_to(len_usize);
                    Some(String::from_utf8_lossy(&bytes).into_owned())
                } else {
                    None
                }
            })
    }

    /// It decodes an optional string. If it's able to read first two bytes which describe
    /// number of bytes and all the bytes then it returns
    /// `Some` with `Vec<u8>`, otherwise `None` would be returned.
    pub fn decode_optional_bytes(src: &mut BytesMut) -> Option<Vec<u8>> {
        src.get(0..BYTES_LEN_LEN)
            .and_then(|len_bytes| len_bytes.to_vec().try_into().ok().map(u16::from_be_bytes))
            .and_then(|len| {
                let len_usize = len as usize;
                if src.len() >= len_usize + BYTES_LEN_LEN {
                    // remove first two bytes as a length of the following string
                    src.advance(BYTES_LEN_LEN);
                    let bytes = src.split_to(len_usize);
                    Some(bytes.to_vec())
                } else {
                    None
                }
            })
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_encode_optional_string_test() {
            let mut buf = BytesMut::new();

            // None
            encode_optional_string(&None::<String>, &mut buf);
            assert_eq!(buf.to_vec(), vec![], "shoud encode None string");

            // Some
            encode_optional_string(&Some(&"foo".to_string()), &mut buf);
            let mut expected: Vec<u8> = vec![0, 3];
            expected.extend_from_slice("foo".as_bytes());
            assert_eq!(buf.to_vec(), expected, "should encode Some string");
        }

        #[test]
        fn test_encode_optional_bytes_test() {
            let mut buf = BytesMut::new();

            // None
            encode_optional_string(&None::<String>, &mut buf);
            assert_eq!(buf.to_vec(), vec![], "shoud encode None bytes");

            // Some
            encode_optional_bytes(&Some(vec![1, 2, 3]), &mut buf);
            let expected: Vec<u8> = vec![0, 3, 1, 2, 3];
            assert_eq!(buf.to_vec(), expected, "should encode Some bytes");
        }

        #[test]
        fn test_decode_optional_string_test() {
            // None
            let decoded = decode_optional_string(&mut BytesMut::from(vec![0, 3].as_slice()));
            assert_eq!(decoded, None, "shoud decode None string");

            // Some
            let mut bytes: Vec<u8> = vec![0, 3];
            bytes.extend_from_slice("foo".as_bytes());
            bytes.extend_from_slice(&[1, 2, 3]);
            let mut buffer = BytesMut::from(bytes.as_slice());
            let decoded = decode_optional_string(&mut buffer);
            assert_eq!(
                decoded,
                Some("foo".to_string()),
                "should encode Some string"
            );
            assert_eq!(buffer.len(), 3, "should leave remaining bytes");
        }

        #[test]
        fn test_decode_optional_bytes_test() {
            // None
            let decoded = decode_optional_bytes(&mut BytesMut::from(vec![0, 3].as_slice()));
            assert_eq!(decoded, None, "shoud decode None bytes");

            // Some
            let mut buffer = BytesMut::from(vec![0, 3, 1, 2, 3, 9, 8, 7].as_slice());
            let decoded = decode_optional_bytes(&mut buffer);
            assert_eq!(decoded, Some(vec![1, 2, 3]), "should encode Some bytes");
            assert_eq!(buffer.len(), 3, "should leave remaining bytes");
        }
    }
}

pub mod getters_setters {
    use crate::v_3_1_1::{packet_id::PacketId, variable::Variable};
    use std::io;

    pub fn get_packet_id(variable: &Variable) -> Option<&PacketId> {
        match variable {
            &Variable::Connack(_) => None,
            &Variable::Connect(_) => None,
            &Variable::Disconnect => None,
            &Variable::Pingreq => None,
            &Variable::Pingresp => None,
            &Variable::Puback(ref variable) => Some(&variable.packet_id),
            &Variable::Pubcomp(ref variable) => Some(&variable.packet_id),
            &Variable::Publish(ref variable) => variable.packet_id.as_ref(),
            &Variable::Pubrec(ref variable) => Some(&variable.packet_id),
            &Variable::Pubrel(ref variable) => Some(&variable.packet_id),
            &Variable::Suback(ref variable) => Some(&variable.packet_id),
            &Variable::Subscribe(ref variable) => Some(&variable.packet_id),
            &Variable::Unsuback(ref variable) => Some(&variable.packet_id),
            &Variable::Unsubscribe(ref variable) => Some(&variable.packet_id),
        }
    }

    pub fn set_packet_id(variable: &mut Variable, packet_id: PacketId) {
        match variable {
            &mut Variable::Connack(_) => {}
            &mut Variable::Connect(_) => {}
            &mut Variable::Disconnect => {}
            &mut Variable::Pingreq => {}
            &mut Variable::Pingresp => {}
            &mut Variable::Puback(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Pubcomp(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Publish(ref mut variable) => {
                variable.packet_id = Some(packet_id);
            }
            &mut Variable::Pubrec(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Pubrel(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Suback(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Subscribe(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Unsuback(ref mut variable) => {
                variable.packet_id = packet_id;
            }
            &mut Variable::Unsubscribe(ref mut variable) => {
                variable.packet_id = packet_id;
            }
        }
    }

    pub fn erase_packet_id(variable: &mut Variable) -> io::Result<()> {
        if let &mut Variable::Publish(ref mut v) = variable {
            v.packet_id = None;
            return Ok(());
        }

        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unable to erase packet id - non-publish variable was received",
        ));
    }
}
