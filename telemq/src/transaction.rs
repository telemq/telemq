use std::time;

use mqtt_packets::v_3_1_1::{publish::fixed_header::get_qos_level, ControlPacket, PacketId, QoS};
use serde::{Deserialize, Deserializer, Serialize};

use crate::session_error::{SessionError, SessionErrorKind, SessionResult};

pub trait CreateTransaction<T>: Sized {
    fn new(packet_id: &PacketId, control_packet: ControlPacket) -> Transaction<T>;
}

pub type TransactionSend = Transaction<TransactionSendState>;
pub type TransactionReceive = Transaction<TransactionReceiveState>;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Transaction<S> {
    pub packet_id: PacketId,
    pub control_packet: ControlPacket,
    pub state: S,
    #[serde(skip_serializing, deserialize_with = "deserialize_time")]
    last_update: time::Instant,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub enum TransactionReceiveState {
    NonAcked,
    PubReled,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub enum TransactionSendState {
    NonAcked,
    PubAcked,
    PubComped,
    PubReced,
}

impl<S> Transaction<S> {
    fn new_inner(packet_id: &PacketId, control_packet: ControlPacket, state: S) -> Transaction<S> {
        Transaction {
            packet_id: packet_id.clone(),
            control_packet,
            state,
            last_update: time::Instant::now(),
        }
    }
}

impl CreateTransaction<TransactionSendState> for TransactionSend {
    fn new(packet_id: &PacketId, control_packet: ControlPacket) -> Self {
        Self::new_inner(packet_id, control_packet, TransactionSendState::NonAcked)
    }
}

impl TransactionSend {
    pub fn puback(&mut self) -> SessionResult<()> {
        let qos = get_qos_level(&self.control_packet.fixed_header).map_err(|_| {
            SessionError::new(
                SessionErrorKind::TransactionError,
                "Unable to extract QoS from a fixed header",
            )
        })?;
        if qos != QoS::One || &self.state != &TransactionSendState::NonAcked {
            return Err(SessionError::new(
                SessionErrorKind::TransactionError,
                "wrong transaction state",
            ));
        }

        self.state = TransactionSendState::PubAcked;
        self.last_update = time::Instant::now();

        Ok(())
    }

    pub fn pubrec(&mut self) -> SessionResult<()> {
        let qos = get_qos_level(&self.control_packet.fixed_header).map_err(|_| {
            SessionError::new(
                SessionErrorKind::TransactionError,
                "Unable to extract QoS from a fixed header",
            )
        })?;
        if qos != QoS::Two || &self.state != &TransactionSendState::NonAcked {
            return Err(SessionError::new(
                SessionErrorKind::TransactionError,
                "wrong transaction state",
            ));
        }

        self.state = TransactionSendState::PubReced;
        self.last_update = time::Instant::now();

        Ok(())
    }

    pub fn pubcomp(&mut self) -> SessionResult<()> {
        let qos = get_qos_level(&self.control_packet.fixed_header).map_err(|_| {
            SessionError::new(
                SessionErrorKind::TransactionError,
                "Unable to extract QoS from a fixed header",
            )
        })?;
        if qos != QoS::Two || &self.state != &TransactionSendState::PubReced {
            return Err(SessionError::new(
                SessionErrorKind::TransactionError,
                "wrong transaction state",
            ));
        }

        self.state = TransactionSendState::PubComped;
        self.last_update = time::Instant::now();

        Ok(())
    }
}

impl CreateTransaction<TransactionReceiveState> for TransactionReceive {
    fn new(packet_id: &PacketId, control_packet: ControlPacket) -> Self {
        Self::new_inner(packet_id, control_packet, TransactionReceiveState::NonAcked)
    }
}

impl TransactionReceive {
    // pub fn is_complete(&self) -> bool {
    //   match (&self.qos, &self.state) {
    //     (QoS::Zero, TransactionReceiveState::NonAcked) => true,
    //     (QoS::One, TransactionReceiveState::NonAcked) => true,
    //     (QoS::Two, TransactionReceiveState::PubReled) => true,
    //     _ => false,
    //   }
    // }

    pub fn pubrel(&mut self) -> SessionResult<()> {
        let qos = get_qos_level(&self.control_packet.fixed_header).map_err(|_| {
            SessionError::new(
                SessionErrorKind::TransactionError,
                "Unable to extract QoS from a fixed header",
            )
        })?;
        if qos != QoS::Two || &self.state != &TransactionReceiveState::NonAcked {
            return Err(SessionError::new(
                SessionErrorKind::TransactionError,
                "wrong transaction state",
            ));
        }

        self.state = TransactionReceiveState::PubReled;
        self.last_update = time::Instant::now();

        Ok(())
    }
}

fn deserialize_time<'de, D>(_deserializer: D) -> Result<time::Instant, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(time::Instant::now())
}

// FIXME:
// #[cfg(test)]
// mod test {
//     use super::*;
//     type TransactionOption<T> = Option<Transaction<T>>;

//     #[test]
//     fn new_ok_send() {
//         let cmd_message = new_command_message(QoS::One);

//         let transaction_opt: TransactionOption<TransactionSendState> =
//             Transaction::new(cmd_message);

//         assert!(transaction_opt.is_some(), "should create some transaction");
//     }

//     #[test]
//     fn new_wrong_type_send() {
//         let transaction_opt: TransactionOption<TransactionSendState> =
//             Transaction::new(CmdMessage::Disconnect);

//         assert!(
//             transaction_opt.is_none(),
//             "should return None for a CmdMessage of a wrong type"
//         );
//     }

//     #[test]
//     fn new_ok_receive() {
//         let cmd_message = new_command_message(QoS::One);

//         let transaction_opt: TransactionOption<TransactionReceiveState> =
//             Transaction::new(cmd_message);

//         assert!(transaction_opt.is_some(), "should create some transaction");
//     }

//     #[test]
//     fn new_wrong_type_receive() {
//         let transaction_opt: TransactionOption<TransactionReceiveState> =
//             Transaction::new(CmdMessage::Disconnect);

//         assert!(
//             transaction_opt.is_none(),
//             "should return None for a CmdMessage of a wrong type"
//         );
//     }

//     fn new_command_message(qos: QoS) -> CmdMessage {
//         CmdMessage::Publish {
//             packet_id: Some(vec![]),
//             topic_name: Topic::try_from("some-topic").unwrap(),
//             is_dup: false,
//             is_retained: false,
//             qos: qos,
//             payload: vec![],
//         }
//     }
// }
