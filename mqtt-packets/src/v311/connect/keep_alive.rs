use bytes::{BufMut, BytesMut};
use std::{convert::TryInto, io, time::Duration};

/// The Keep Alive is a time interval measured in seconds.
/// Expressed as a 16-bit word, it is the maximum
/// time interval that is permitted to elapse between the point at which
/// the Client finishes transmitting one
/// Control Packet and the point it starts sending the next.
#[derive(Debug, PartialEq, Clone)]
pub struct KeepAlive(Duration);

impl KeepAlive {
    // Number of bits which represents `KeepAlive`.
    const KEEP_ALIVE_LEN: usize = 2;

    /// Constructor function for `KeepAlive` that takes seconds as an argument.
    pub fn new(secs: u16) -> KeepAlive {
        KeepAlive(Duration::from_secs(secs as u64))
    }

    /// It return a number of second for keep alive duration.
    pub fn as_secs(&mut self) -> u64 {
        self.0.as_secs()
    }

    pub fn as_duration(&self) -> Duration {
        self.0
    }
}

impl Default for KeepAlive {
    fn default() -> KeepAlive {
        KeepAlive::new(0)
    }
}

/// `KeepAlive` Tokio codec.
pub struct KeepAliveCodec;

impl KeepAliveCodec {
    /// Constructor function for `KeepAliveCodec`.
    pub fn new() -> KeepAliveCodec {
        KeepAliveCodec {}
    }

    pub fn encode(&mut self, item: &KeepAlive, dst: &mut BytesMut) -> Result<(), std::io::Error> {
        let duration = item.0.as_secs();
        if duration > (::std::u16::MAX as u64) {
            return Err(::std::io::Error::new(
                ::std::io::ErrorKind::Other,
                "Unable to encode keep alive duration - too big.",
            ));
        }
        dst.put_u16(duration as u16);
        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<KeepAlive>, std::io::Error> {
        if src.len() < KeepAlive::KEEP_ALIVE_LEN {
            return Ok(None);
        }
        let keep_alive_bytes = src.split_to(KeepAlive::KEEP_ALIVE_LEN);
        if keep_alive_bytes.len() == KeepAlive::KEEP_ALIVE_LEN {
            let keep_alive = keep_alive_bytes
                .to_vec()
                .try_into()
                .map(u16::from_be_bytes)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", err)))?;
            Ok(Some(KeepAlive::new(keep_alive)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keep_alive_new() {
        let secs: u16 = 500;
        assert_eq!(KeepAlive::new(secs).0.as_secs(), 500);
    }

    #[test]
    fn test_keep_alive_as_secs() {
        let secs: u16 = 500;
        assert_eq!(KeepAlive::new(secs).as_secs(), 500);
    }

    #[test]
    fn test_keep_alive_default() {
        let keep_alive: KeepAlive = Default::default();
        assert_eq!(keep_alive.0.as_secs(), 0);
    }

    #[test]
    fn test_keep_alive_encode() {
        let mut codec = KeepAliveCodec::new();
        let keep_alive = KeepAlive::new(3);
        let mut buf = BytesMut::new();

        codec
            .encode(&keep_alive, &mut buf)
            .expect("Should encode keep alive duration without errors.");
        assert_eq!(buf.to_vec(), vec![0, 3]);
    }

    #[test]
    fn test_keep_alive_decode() {
        let mut codec = KeepAliveCodec::new();
        let keep_alive = KeepAlive::new(3);
        let mut buf = BytesMut::from(vec![0, 3].as_slice());

        let keep_alive_option = codec
            .decode(&mut buf)
            .expect("Should encode keep alive duration without errors.");
        assert_eq!(keep_alive_option.unwrap(), keep_alive);
    }

    #[test]
    fn test_keep_alive_decode_not_enough() {
        let mut codec = KeepAliveCodec::new();
        let mut buf = BytesMut::from(vec![0].as_slice());

        let keep_alive_option = codec
            .decode(&mut buf)
            .expect("Should encode keep alive duration without errors.");
        assert!(
            keep_alive_option.is_none(),
            "Should be None if there is not enough bytes in a buffer"
        );
    }
}
