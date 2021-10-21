// Copyright 2017 TeleMQ contributors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::collections::{HashMap, VecDeque};
use std::mem::replace as mem_replace;

use mqtt_packets::v_3_1_1::{
    publish::fixed_header::{get_qos_level, set_dup},
    subscribe::topic_subscription::TopicSubscription,
    topic::{Subscription, Topic},
    utils::getters_setters,
    ControlPacket, PacketId, QoS,
};
use serde::{Deserialize, Serialize};

use super::connection_provider::SessionConnectionProvider;
use super::session_error::*;
use super::transaction::{CreateTransaction, TransactionReceive, TransactionSend};

/// Client session state.
#[derive(Clone, Debug)]
pub enum SessionState {
    /// `NonConnected` means client has opened connection however
    /// Connect packet wasn't sent so no other packet can't be
    /// received from a user.
    NonConnected,

    /// `Connected` state represent a state of client
    /// when Connect frame was received, authentication successfully
    /// completed, server can exchange packets with a client.
    Connected(SessionConnectedState),

    /// Session is closed for receiving packets from a client.
    Closed,
}

impl SessionState {
    pub fn is_non_connected(&self) -> bool {
        match self {
            SessionState::NonConnected => true,
            _ => false,
        }
    }

    pub fn is_connected(&self) -> bool {
        match self {
            SessionState::Connected(_) => true,
            _ => false,
        }
    }

    pub fn into_connected(&mut self, connection_provider: SessionConnectionProvider) {
        *self = Self::connected(connection_provider);
    }

    pub fn make_connected(&mut self, connected_state: SessionConnectedState) {
        *self = SessionState::Connected(connected_state);
    }

    pub fn into_closed(&mut self) -> SessionResult<SessionConnectedState> {
        match self {
            SessionState::NonConnected | SessionState::Closed => {
                return Err(SessionError::new(
                    SessionErrorKind::WrongState,
                    format!(
                        "Unable to move session into a closed state. Current state {:?}",
                        self
                    ),
                ));
            }
            SessionState::Connected(_) => {
                // continue
            }
        };

        let connected_state = mem_replace(self, SessionState::Closed);

        if let SessionState::Connected(cs) = connected_state {
            return Ok(cs);
        }

        // should not be reachable
        return Err(SessionError::new(
            SessionErrorKind::WrongState,
            format!("Unexpected error. Current state {:?}", self),
        ));
    }

    pub fn get_client_id(&self) -> SessionResult<String> {
        match self {
            SessionState::Connected(SessionConnectedState { client_id, .. }) => {
                return Ok(client_id.clone())
            }
            _ => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Unable to get a client id for a client which is not connected",
            )),
        }
    }

    pub fn has_clean_session(&self) -> bool {
        match self {
            SessionState::Connected(SessionConnectedState { clean_session, .. }) => {
                return *clean_session;
            }
            _ => true,
        }
    }

    pub fn connected(
        SessionConnectionProvider {
            client_id,
            clean_session,
            will_topic,
            will_message,
            will_qos,
        }: SessionConnectionProvider,
    ) -> SessionState {
        SessionState::Connected(SessionConnectedState::new(
            client_id,
            clean_session,
            will_topic,
            will_message,
            will_qos,
        ))
    }

    pub fn create_receive_transaction_from_packet(
        &mut self,
        control_packet: ControlPacket,
    ) -> SessionResult<QoS> {
        let qos = get_qos_level(&control_packet.fixed_header).map_err(|_| {
            SessionError::new(
                SessionErrorKind::TransactionError,
                "Unable to extract QoS from a fixed header",
            )
        })?;

        if qos == QoS::Zero {
            return Ok(qos);
        }

        let packet_id = match getters_setters::get_packet_id(&control_packet.variable) {
            Some(packet_id) => packet_id,
            None => {
                return Err(SessionError::new(
                    SessionErrorKind::MqttPolicyError,
                    "No packet id found of a publish packet with QoS level greater than 0",
                ));
            }
        };

        self.create_receive_transaction(&packet_id, control_packet.clone())?;

        return Ok(qos);
    }

    pub fn create_receive_transaction(
        &mut self,
        packet_id: &PacketId,
        control_packet: ControlPacket,
    ) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot accept publish in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_received_not_acked,
                ..
            }) => {
                let transaction = TransactionReceive::new(packet_id, control_packet);
                messages_received_not_acked.insert(packet_id.clone(), transaction);
                Ok(())
            }
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot accept publish in a closed session",
            )),
        }
    }

    pub fn create_send_transaction_from_packet(
        &mut self,
        control_packet: &ControlPacket,
    ) -> SessionResult<Option<Vec<u8>>> {
        let qos = get_qos_level(&control_packet.fixed_header).map_err(|_| {
            SessionError::new(
                SessionErrorKind::TransactionError,
                "Unable to extract QoS from a fixed header",
            )
        })?;

        if qos == QoS::Zero {
            return Ok(None);
        }

        let packet_id = self.generate_packet_id()?;

        self.create_send_transaction(&packet_id, control_packet.clone())?;

        return Ok(Some(packet_id));
    }

    pub fn get_queued_messages(&mut self) -> VecDeque<ControlPacket> {
        if let SessionState::Connected(ref mut connected_state) = self {
            return mem_replace(
                &mut connected_state.messages_pending_transmition,
                VecDeque::new(),
            );
        }
        VecDeque::new()
    }

    // generates a unique packet id for a send transaction
    fn generate_packet_id(&self) -> SessionResult<PacketId> {
        let mut packet_id = vec![0u8, 0u8];

        loop {
            if !self.check_packet_id(&packet_id) {
                break;
            }

            match packet_id[1].checked_add(1) {
                Some(sum) => {
                    packet_id[1] = sum;
                }
                None => {
                    packet_id[1] = 0;
                    match packet_id[0].checked_add(1) {
                        Some(sum) => {
                            packet_id[0] = sum;
                        }
                        None => {
                            return Err(SessionError::new(
                                SessionErrorKind::MqttPolicyError,
                                "Unable to generate unique packet id",
                            ));
                        }
                    }
                }
            }
        }

        Ok(packet_id)
    }

    pub fn create_send_transaction(
        &mut self,
        packet_id: &PacketId,
        mut control_packet: ControlPacket,
    ) -> SessionResult<()> {
        set_dup(&mut control_packet.fixed_header, true);
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot accept publish in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_sent_not_acked,
                ..
            }) => {
                let transaction = TransactionSend::new(packet_id, control_packet);
                messages_sent_not_acked.insert(packet_id.clone(), transaction);
                Ok(())
            }
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot accept publish in a closed session",
            )),
        }
    }

    pub fn puback(&mut self, packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot puback transaction in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_sent_not_acked,
                ..
            }) => match messages_sent_not_acked.get_mut(packet_id) {
                Some(transaction) => {
                    transaction.puback()?;
                    messages_sent_not_acked.remove(packet_id);
                    Ok(())
                }
                None => Err(SessionError::new(
                    SessionErrorKind::TransactionError,
                    format!(
                        "Puback: cannot find transaction for a given packet id {:?}",
                        packet_id
                    ),
                )),
            },
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot puback transaction in a closed session",
            )),
        }
    }

    pub fn pubacked(&mut self, packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot mark transaction as pubacked in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_received_not_acked,
                ..
            }) => {
                messages_received_not_acked.remove(packet_id);
                Ok(())
            }
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot mark transaction as pubacked in a closed session",
            )),
        }
    }

    pub fn pubcomp(&mut self, packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot pubcomp transaction in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_sent_not_acked,
                ..
            }) => match messages_sent_not_acked.get_mut(packet_id) {
                Some(transaction) => {
                    let r = transaction.pubcomp();
                    messages_sent_not_acked.remove(packet_id);
                    r
                }
                None => Err(SessionError::new(
                    SessionErrorKind::TransactionError,
                    format!(
                        "Pubcomp: cannot find transaction for a given packet id {:?}",
                        packet_id
                    ),
                )),
            },
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot pubcomp transaction in a closed session",
            )),
        }
    }

    pub fn pubcomped(&mut self, packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot mark transaction as pubcomped in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_received_not_acked,
                ..
            }) => {
                messages_received_not_acked.remove(packet_id);
                Ok(())
            }
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot mark transaction as pubcomped in a closed session",
            )),
        }
    }

    pub fn pubrec(&mut self, packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot pubrec transaction in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_sent_not_acked,
                ..
            }) => match messages_sent_not_acked.get_mut(packet_id) {
                Some(transaction) => transaction.pubrec(),
                None => Err(SessionError::new(
                    SessionErrorKind::TransactionError,
                    format!(
                        "Pubrec: cannot find transaction for a given packet id {:?} ({:?})",
                        packet_id, messages_sent_not_acked
                    ),
                )),
            },
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot pubcomp transaction in a closed session",
            )),
        }
    }

    pub fn pubreced(&mut self, _packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot mark transaction as pubreced in a non-connected session",
            )),
            SessionState::Connected(_) => Ok(()),
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot mark transaction as pubreced in a closed session",
            )),
        }
    }

    pub fn pubrel(&mut self, packet_id: &PacketId) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot pubrel transaction in a non-connected session",
            )),
            SessionState::Connected(SessionConnectedState {
                messages_received_not_acked,
                ..
            }) => match messages_received_not_acked.get_mut(packet_id) {
                Some(transaction) => transaction.pubrel(),
                None => Err(SessionError::new(
                    SessionErrorKind::TransactionError,
                    format!(
                        "Pubrel: cannot find transaction for a given packet id {:?}",
                        packet_id
                    ),
                )),
            },
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot pubrel transaction in a closed session",
            )),
        }
    }

    pub fn subscribe(&mut self, subscriptions: Vec<TopicSubscription>) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot subscribe in a non-connected session",
            )),
            SessionState::Connected(connected_state) => {
                for sub in subscriptions {
                    connected_state.add_or_replace_subscription(sub);
                }

                Ok(())
            }
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot subscribe in a closed session",
            )),
        }
    }

    pub fn unsubscribe(&mut self, to_unsubscribe: Vec<Subscription>) -> SessionResult<()> {
        match self {
            SessionState::NonConnected => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot unsubscribe in a non-connected session",
            )),
            SessionState::Connected(connected_state) => {
                for sub in to_unsubscribe {
                    connected_state.remove_subscription(sub);
                }

                Ok(())
            }
            SessionState::Closed => Err(SessionError::new(
                SessionErrorKind::WrongState,
                "Cannot unsubscribe in a closed session",
            )),
        }
    }

    pub fn check_packet_id(&self, packet_id: &PacketId) -> bool {
        match self {
            SessionState::Connected(ref connected_session) => {
                connected_session.check_packet_id(packet_id)
            }
            _ => false,
        }
    }

    pub fn get_subscription_qoss(&self, topic: &Topic) -> Vec<QoS> {
        match self {
            SessionState::Connected(ref connected_session) => connected_session
                .subscriptions
                .iter()
                .filter_map(|(qos, subscription)| {
                    if subscription.topic_matches(topic) {
                        Some(qos.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => vec![],
        }
    }

    pub fn get_topic_subscriptin_qos(&self, original: String) -> Option<QoS> {
        match self {
            SessionState::Connected(ref connected_session) => connected_session
                .subscriptions
                .iter()
                .find_map(|(qos, sub)| {
                    if sub.original == original {
                        Some(qos.clone())
                    } else {
                        None
                    }
                }),
            _ => None,
        }
    }

    pub fn get_will_data(&mut self) -> Option<(Topic, QoS, Vec<u8>, bool)> {
        if let SessionState::Connected(ref mut connected_state) = self {
            if let (Some(topic), Some(qos), Some(message)) = (
                connected_state.will_topic.take(),
                connected_state.will_qos.take(),
                connected_state.will_message.take(),
            ) {
                return Some((topic, qos, message, connected_state.will_retain));
            }
        }
        None
    }
}

/// Connected client session.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SessionConnectedState {
    pub client_id: String,

    /// If `clean_session` is set to `false`, the Server MUST resume communications with the Client based on state from
    /// the current Session (as identified by the Client identifier). If there is no Session associated with the Client
    /// identifier the Server MUST create a new Session. The Client and Server MUST store the Session after
    /// the Client and Server are disconnected. After the disconnection of a Session that had
    /// `clean_session` set to false, the Server MUST store further QoS 1 and QoS 2 messages that match any
    /// subscriptions that the client had at the time of disconnection as part of the Session state.
    /// It MAY also store QoS 0 messages that meet the same criteria.
    ///
    /// If `clean_session` is set to `true`, the Client and Server MUST discard any previous Session and start a new
    /// one. This Session lasts as long as the Network Connection. State data associated with this Session
    /// MUST NOT be reused in any subsequent Session.
    pub clean_session: bool,

    /// List of subscriptions along with QoS.
    pub subscriptions: Vec<(QoS, Subscription)>,

    /// QoS 1 and QoS 2 messages which have been sent to the Client, but have not been completely
    /// acknowledged.
    pub messages_sent_not_acked: HashMap<PacketId, TransactionSend>,

    /// QoS 1 and QoS 2 messages pending transmission to the Client.
    pub messages_pending_transmition: VecDeque<ControlPacket>,

    /// QoS 2 messages which have been received from the Client, but have not been completely
    /// acknowledged.
    pub messages_received_not_acked: HashMap<PacketId, TransactionReceive>,

    pub will_flag: bool,

    /// Topic of Will Message
    pub will_topic: Option<Topic>,

    /// Will Message content
    pub will_message: Option<Vec<u8>>,

    /// QoS of Will Message
    pub will_qos: Option<QoS>,

    /// If set to true Will Message will be published
    /// as a retained message.
    pub will_retain: bool,
}

impl SessionConnectedState {
    pub fn add_or_replace_subscription(&mut self, subscription: TopicSubscription) {
        let existing_subscription = self
            .subscriptions
            .iter()
            .position(|sub| sub.1.original == subscription.topic_filter.original);

        match existing_subscription {
            Some(idx) => {
                self.subscriptions[idx] = (subscription.qos, subscription.topic_filter);
            }
            None => {
                self.subscriptions
                    .push((subscription.qos, subscription.topic_filter));
            }
        }
    }

    pub fn remove_subscription(&mut self, subscription: Subscription) {
        self.subscriptions.retain(|(_, sub)| *sub != subscription);
    }

    pub fn check_packet_id(&self, packet_id: &PacketId) -> bool {
        self.messages_sent_not_acked.contains_key(packet_id)
            || self.messages_received_not_acked.contains_key(packet_id)
    }

    pub fn new(
        client_id: String,
        clean_session: bool,
        will_topic: Option<Topic>,
        will_message: Option<Vec<u8>>,
        will_qos: Option<QoS>,
    ) -> Self {
        SessionConnectedState {
            client_id,
            clean_session,
            will_topic,
            will_message,
            will_qos,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod test_connected_state {
    use super::*;

    #[test]
    fn add_subscription() {
        let mut state = SessionConnectedState {
            client_id: "someid".into(),
            clean_session: true,
            messages_pending_transmition: VecDeque::new(),
            messages_received_not_acked: HashMap::new(),
            messages_sent_not_acked: HashMap::new(),
            subscriptions: Vec::new(),
            will_flag: false,
            will_message: None,
            will_qos: None,
            will_retain: false,
            will_topic: None,
        };
        let subscription = TopicSubscription {
            qos: QoS::Zero,
            topic_filter: Subscription::try_from("sub").unwrap(),
        };

        let subscription_clone = subscription.clone();

        state.add_or_replace_subscription(subscription);

        assert_eq!(
            state.subscriptions.len(),
            1,
            "new subscription should be added"
        );
        assert_eq!(
            state.subscriptions[0].0, subscription_clone.qos,
            "should add a subscription witha a proer QoS"
        );
        assert_eq!(
            state.subscriptions[0].1.original, subscription_clone.topic_filter.original,
            "should add a subscription with a proper topic"
        );
    }

    #[test]
    fn replace_subscription() {
        let mut state = SessionConnectedState {
            client_id: "someid".into(),
            clean_session: true,
            messages_pending_transmition: VecDeque::new(),
            messages_received_not_acked: HashMap::new(),
            messages_sent_not_acked: HashMap::new(),
            subscriptions: vec![(QoS::Zero, Subscription::try_from("sub").unwrap())],
            will_flag: false,
            will_message: None,
            will_qos: None,
            will_retain: false,
            will_topic: None,
        };
        let subscription = TopicSubscription {
            qos: QoS::One,
            topic_filter: Subscription::try_from("sub").unwrap(),
        };

        let subscription_clone = subscription.clone();

        state.add_or_replace_subscription(subscription);

        assert_eq!(
            state.subscriptions.len(),
            1,
            "replace existing subscription"
        );
        assert_eq!(
            state.subscriptions[0].0, subscription_clone.qos,
            "should add a subscription witha a proer QoS"
        );
        assert_eq!(
            state.subscriptions[0].1.original, subscription_clone.topic_filter.original,
            "should add a subscription with a proper topic"
        );
    }
}
