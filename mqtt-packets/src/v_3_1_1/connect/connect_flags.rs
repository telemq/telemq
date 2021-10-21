use crate::v_3_1_1::QoS;

use bytes::{BufMut, BytesMut};

/// The Connect Flags byte contains a number of parameters specifying the behavior of the MQTT
/// connection. It also indicates the presence or absence of fields in the payload.
#[derive(Debug, PartialEq, Clone)]
pub struct ConnectFlags(u8);

impl ConnectFlags {
    const RESERVED_MASK: u8 = 0b00000001;
    const CLEAN_SESSION_MASK: u8 = 0b00000010;
    const WILL_FLAG_MASK: u8 = 0b00000100;
    const QOS_MASK: u8 = 0b00011000;
    const WILL_RETAIN_MASK: u8 = 0b00100000;
    const USERNAME_MASK: u8 = 0b01000000;
    const PASSWORD_MASK: u8 = 0b10000000;
    const CONNECT_FLAG_LEN: usize = 1;

    /// `ConnectFlags`
    pub fn new(flag: u8) -> ConnectFlags {
        ConnectFlags(flag)
    }

    // TODO: add test
    pub fn is_reserved_set_to_0(&self) -> bool {
        self.0 & Self::RESERVED_MASK == 0
    }

    /// It returns true if `ConnectFlags` contains non-zero clean session bit.
    pub fn has_clean_session(&self) -> bool {
        self.has_active_bit(Self::CLEAN_SESSION_MASK)
    }

    /// Depending on given `active` arguments it sets clean session bit either to zero
    /// or to one.
    pub fn set_clean_session(&mut self, active: bool) {
        self.set_bits(Self::CLEAN_SESSION_MASK, active);
    }

    /// It returns true if `ConnectFlags` contains non-zero will flag bit.
    pub fn has_will_flag(&self) -> bool {
        self.has_active_bit(Self::WILL_FLAG_MASK)
    }

    /// Depending on given `active` arguments it sets will flag bit either to zero
    /// or to one.
    pub fn set_will_flag(&mut self, active: bool) {
        self.set_bits(Self::WILL_FLAG_MASK, active);
    }

    /// It returns QoS flag value.
    pub fn qos_value(&self) -> std::io::Result<QoS> {
        let bits = (self.0 & Self::QOS_MASK).rotate_right(3);
        QoS::try_from(bits)
    }

    /// Depending on provided `QoS` it sets related flag bits.
    pub fn set_qos_value(&mut self, qos: &QoS) {
        // remove previous value
        self.0 &= !Self::QOS_MASK;
        // set new value
        self.0 |= qos.bits().rotate_left(3);
    }

    /// It returns true if `ConnectFlags` contains non-zero will retain bit.
    pub fn has_will_retain(&self) -> bool {
        self.has_active_bit(Self::WILL_RETAIN_MASK)
    }

    /// Depending on given `active` arguments it sets will retain bit either to zero
    /// or to one.
    pub fn set_will_retain(&mut self, active: bool) {
        self.set_bits(Self::WILL_RETAIN_MASK, active);
    }

    /// It returns true if `ConnectFlags` contains non-zero username bit.
    pub fn has_username(&self) -> bool {
        self.has_active_bit(Self::USERNAME_MASK)
    }

    /// Depending on given `active` arguments it sets username bit either to zero
    /// or to one.
    pub fn set_username(&mut self, active: bool) {
        self.set_bits(Self::USERNAME_MASK, active);
    }

    /// It returns true if `ConnectFlags` contains non-zero password bit.
    pub fn has_password(&self) -> bool {
        self.has_active_bit(Self::PASSWORD_MASK)
    }

    /// Depending on given `active` arguments it sets password bit either to zero
    /// or to one.
    pub fn set_password(&mut self, active: bool) {
        self.set_bits(Self::PASSWORD_MASK, active);
    }

    /// Depending on given `active` arguments it sets a bit either to zero
    /// or to one, basing on active mask.
    fn set_bits(&mut self, active_mask: u8, active: bool) {
        if active {
            self.0 |= active_mask;
        } else {
            self.0 &= !active_mask;
        }
    }

    fn has_active_bit(&self, active_mask: u8) -> bool {
        self.0 & active_mask == active_mask
    }
}

pub struct ConnectFlagsCodec;

impl ConnectFlagsCodec {
    pub fn new() -> ConnectFlagsCodec {
        ConnectFlagsCodec {}
    }

    pub fn encode(
        &mut self,
        item: &ConnectFlags,
        dst: &mut BytesMut,
    ) -> Result<(), std::io::Error> {
        dst.put_u8(item.0);
        Ok(())
    }

    pub fn decode(&mut self, src: &mut BytesMut) -> Result<Option<ConnectFlags>, std::io::Error> {
        if src.len() < ConnectFlags::CONNECT_FLAG_LEN {
            return Ok(None);
        }
        let flag_byte = src.split_to(ConnectFlags::CONNECT_FLAG_LEN);

        Ok(flag_byte.first().map(|b| ConnectFlags::new(*b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        assert_eq!(ConnectFlags::new(0).0, 0);
    }

    #[test]
    fn test_has_clean_session() {
        assert!(!ConnectFlags::new(0).has_clean_session());
        assert!(ConnectFlags(0b00000010).has_clean_session());
    }

    #[test]
    fn test_set_clean_session() {
        let mut flags = ConnectFlags::new(0);
        flags.set_clean_session(true);
        assert!(flags.has_clean_session());
        flags.set_clean_session(false);
        assert!(!flags.has_clean_session());
    }

    #[test]
    fn test_has_will_flag() {
        assert!(!ConnectFlags::new(0).has_will_flag());
        assert!(ConnectFlags(0b00000100).has_will_flag());
    }

    #[test]
    fn test_set_will_flag() {
        let mut flags = ConnectFlags::new(0);
        flags.set_will_flag(true);
        assert!(flags.has_will_flag());
        flags.set_will_flag(false);
        assert!(!flags.has_will_flag());
    }

    #[test]
    fn test_set_qos_value() {
        let mut flags = ConnectFlags::new(0);
        flags.set_qos_value(&QoS::Two);
        assert_eq!(flags, ConnectFlags(0b00010000));
    }

    #[test]
    fn test_qos_value() {
        assert_eq!(ConnectFlags(0b00010000).qos_value().unwrap(), QoS::Two);
    }

    #[test]
    fn test_has_will_retain() {
        assert!(!ConnectFlags::new(0).has_will_retain());
        assert!(ConnectFlags(0b00100000).has_will_retain());
    }

    #[test]
    fn test_set_will_retain() {
        let mut flags = ConnectFlags::new(0);
        flags.set_will_retain(true);
        assert!(flags.has_will_retain());
        flags.set_will_retain(false);
        assert!(!flags.has_will_retain());
    }

    #[test]
    fn test_has_username() {
        assert!(!ConnectFlags::new(0).has_username());
        assert!(ConnectFlags(0b01000000).has_username());
    }

    #[test]
    fn test_set_username() {
        let mut flags = ConnectFlags::new(0);
        flags.set_username(true);
        assert!(flags.has_username());
        flags.set_username(false);
        assert!(!flags.has_username());
    }

    #[test]
    fn test_has_password() {
        assert!(!ConnectFlags::new(0).has_password());
        assert!(ConnectFlags(0b10000000).has_password());
    }

    #[test]
    fn test_set_password() {
        let mut flags = ConnectFlags::new(0);
        flags.set_password(true);
        assert!(flags.has_password());
        flags.set_password(false);
        assert!(!flags.has_password());
    }

    #[test]
    fn test_mix_flags() {
        let mut flags = ConnectFlags::new(0);
        flags.set_qos_value(&QoS::Two);
        flags.set_will_flag(true);
        assert_eq!(flags, ConnectFlags(0b00010100));
    }

    #[test]
    fn test_encode() {
        let mut codec = ConnectFlagsCodec::new();
        let mut buf = BytesMut::new();
        let flags = ConnectFlags::new(0b00010100);
        codec
            .encode(&flags, &mut buf)
            .expect("Should encode connection flags without errors");
        assert_eq!(buf.to_vec(), vec![0b00010100]);
    }

    #[test]
    fn test_decode() {
        let mut codec = ConnectFlagsCodec::new();
        let mut buf = BytesMut::from(vec![0b00010100].as_slice());
        let flags_opt = codec
            .decode(&mut buf)
            .expect("Should decode connection flag without errors");
        assert_eq!(flags_opt.unwrap(), ConnectFlags(0b00010100));
    }
}
