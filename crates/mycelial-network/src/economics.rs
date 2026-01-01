//! Economics protocol handlers for Mycelial Network
//!
//! This module provides handlers for the economics gossipsub protocols:
//! - Vouch: Reputation vouching and delegation
//! - Credit: Mutual credit lines and transfers
//! - Governance: Proposals and voting
//! - Resource: Resource sharing metrics

use mycelial_protocol::{topics, CreditMessage, GovernanceMessage, ResourceMessage, VouchMessage};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::error::{NetworkError, Result};
use crate::event::NetworkEvent;
use crate::service::NetworkHandle;

/// Economics protocol event types
#[derive(Debug, Clone)]
pub enum EconomicsEvent {
    /// Vouch protocol event
    Vouch(VouchMessage),
    /// Credit protocol event
    Credit(CreditMessage),
    /// Governance protocol event
    Governance(GovernanceMessage),
    /// Resource protocol event
    Resource(ResourceMessage),
}

/// Handler for economics protocol messages
pub struct EconomicsHandler {
    /// Network handle for publishing
    network: NetworkHandle,
    /// Event sender for economics events
    event_tx: broadcast::Sender<EconomicsEvent>,
}

impl EconomicsHandler {
    /// Create a new economics handler
    pub fn new(network: NetworkHandle) -> (Self, broadcast::Receiver<EconomicsEvent>) {
        let (event_tx, event_rx) = broadcast::channel(256);
        (Self { network, event_tx }, event_rx)
    }

    /// Handle a network event, parsing economics messages
    pub fn handle_network_event(&self, event: &NetworkEvent) -> Option<EconomicsEvent> {
        if let NetworkEvent::MessageReceived { topic, data, .. } = event {
            match topic.as_str() {
                t if t == topics::VOUCH => match serde_json::from_slice::<VouchMessage>(data) {
                    Ok(msg) => {
                        debug!("Received vouch message: {:?}", msg);
                        let event = EconomicsEvent::Vouch(msg);
                        let _ = self.event_tx.send(event.clone());
                        return Some(event);
                    }
                    Err(e) => warn!("Failed to parse vouch message: {}", e),
                },
                t if t == topics::CREDIT => match serde_json::from_slice::<CreditMessage>(data) {
                    Ok(msg) => {
                        debug!("Received credit message: {:?}", msg);
                        let event = EconomicsEvent::Credit(msg);
                        let _ = self.event_tx.send(event.clone());
                        return Some(event);
                    }
                    Err(e) => warn!("Failed to parse credit message: {}", e),
                },
                t if t == topics::GOVERNANCE => {
                    match serde_json::from_slice::<GovernanceMessage>(data) {
                        Ok(msg) => {
                            debug!("Received governance message: {:?}", msg);
                            let event = EconomicsEvent::Governance(msg);
                            let _ = self.event_tx.send(event.clone());
                            return Some(event);
                        }
                        Err(e) => warn!("Failed to parse governance message: {}", e),
                    }
                }
                t if t == topics::RESOURCE => {
                    match serde_json::from_slice::<ResourceMessage>(data) {
                        Ok(msg) => {
                            debug!("Received resource message: {:?}", msg);
                            let event = EconomicsEvent::Resource(msg);
                            let _ = self.event_tx.send(event.clone());
                            return Some(event);
                        }
                        Err(e) => warn!("Failed to parse resource message: {}", e),
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Publish a vouch message
    pub async fn publish_vouch(&self, msg: &VouchMessage) -> Result<()> {
        let data =
            serde_json::to_vec(msg).map_err(|e| NetworkError::Serialization(e.to_string()))?;
        self.network.publish(topics::VOUCH, data).await
    }

    /// Publish a credit message
    pub async fn publish_credit(&self, msg: &CreditMessage) -> Result<()> {
        let data =
            serde_json::to_vec(msg).map_err(|e| NetworkError::Serialization(e.to_string()))?;
        self.network.publish(topics::CREDIT, data).await
    }

    /// Publish a governance message
    pub async fn publish_governance(&self, msg: &GovernanceMessage) -> Result<()> {
        let data =
            serde_json::to_vec(msg).map_err(|e| NetworkError::Serialization(e.to_string()))?;
        self.network.publish(topics::GOVERNANCE, data).await
    }

    /// Publish a resource message
    pub async fn publish_resource(&self, msg: &ResourceMessage) -> Result<()> {
        let data =
            serde_json::to_vec(msg).map_err(|e| NetworkError::Serialization(e.to_string()))?;
        self.network.publish(topics::RESOURCE, data).await
    }
}

/// Parse a network message into an economics event
pub fn parse_economics_message(topic: &str, data: &[u8]) -> Option<EconomicsEvent> {
    match topic {
        t if t == topics::VOUCH => serde_json::from_slice::<VouchMessage>(data)
            .ok()
            .map(EconomicsEvent::Vouch),
        t if t == topics::CREDIT => serde_json::from_slice::<CreditMessage>(data)
            .ok()
            .map(EconomicsEvent::Credit),
        t if t == topics::GOVERNANCE => serde_json::from_slice::<GovernanceMessage>(data)
            .ok()
            .map(EconomicsEvent::Governance),
        t if t == topics::RESOURCE => serde_json::from_slice::<ResourceMessage>(data)
            .ok()
            .map(EconomicsEvent::Resource),
        _ => None,
    }
}

/// Check if a topic is an economics topic
pub fn is_economics_topic(topic: &str) -> bool {
    topic == topics::VOUCH
        || topic == topics::CREDIT
        || topic == topics::GOVERNANCE
        || topic == topics::RESOURCE
}

/// Get all economics topic names
pub fn economics_topics() -> &'static [&'static str] {
    &[
        topics::VOUCH,
        topics::CREDIT,
        topics::GOVERNANCE,
        topics::RESOURCE,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mycelial_protocol::{
        CreateCreditLine, CreateProposal, ResourceContribution, ResourceType, VouchRequest,
    };

    #[test]
    fn test_is_economics_topic() {
        assert!(is_economics_topic("/mycelial/1.0.0/vouch"));
        assert!(is_economics_topic("/mycelial/1.0.0/credit"));
        assert!(is_economics_topic("/mycelial/1.0.0/governance"));
        assert!(is_economics_topic("/mycelial/1.0.0/resource"));
        assert!(!is_economics_topic("/mycelial/1.0.0/chat"));
        assert!(!is_economics_topic("/mycelial/1.0.0/direct"));
    }

    #[test]
    fn test_parse_vouch_message() {
        let msg = VouchMessage::VouchRequest(VouchRequest::new(
            "alice".to_string(),
            "bob".to_string(),
            0.5,
        ));
        let data = serde_json::to_vec(&msg).unwrap();

        let parsed = parse_economics_message(topics::VOUCH, &data);
        assert!(parsed.is_some());
        if let Some(EconomicsEvent::Vouch(VouchMessage::VouchRequest(v))) = parsed {
            assert_eq!(v.voucher, "alice");
            assert_eq!(v.vouchee, "bob");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_parse_credit_message() {
        let msg = CreditMessage::CreateLine(CreateCreditLine::new(
            "alice".to_string(),
            "bob".to_string(),
            100.0,
        ));
        let data = serde_json::to_vec(&msg).unwrap();

        let parsed = parse_economics_message(topics::CREDIT, &data);
        assert!(parsed.is_some());
        if let Some(EconomicsEvent::Credit(CreditMessage::CreateLine(c))) = parsed {
            assert_eq!(c.creditor, "alice");
            assert_eq!(c.limit, 100.0);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_parse_governance_message() {
        let msg = GovernanceMessage::CreateProposal(CreateProposal::new(
            "alice".to_string(),
            "Network Upgrade".to_string(),
            "Upgrade to v2.0".to_string(),
        ));
        let data = serde_json::to_vec(&msg).unwrap();

        let parsed = parse_economics_message(topics::GOVERNANCE, &data);
        assert!(parsed.is_some());
        if let Some(EconomicsEvent::Governance(GovernanceMessage::CreateProposal(p))) = parsed {
            assert_eq!(p.proposer, "alice");
            assert_eq!(p.title, "Network Upgrade");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_parse_resource_message() {
        let msg = ResourceMessage::Contribution(ResourceContribution::new(
            "alice".to_string(),
            ResourceType::Bandwidth,
            1000.0,
            "Mbps".to_string(),
        ));
        let data = serde_json::to_vec(&msg).unwrap();

        let parsed = parse_economics_message(topics::RESOURCE, &data);
        assert!(parsed.is_some());
        if let Some(EconomicsEvent::Resource(ResourceMessage::Contribution(r))) = parsed {
            assert_eq!(r.peer_id, "alice");
            assert_eq!(r.resource_type, ResourceType::Bandwidth);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_parse_invalid_topic() {
        let data = b"some data";
        let parsed = parse_economics_message("/mycelial/1.0.0/chat", data);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_parse_invalid_data() {
        let data = b"not json";
        let parsed = parse_economics_message(topics::VOUCH, data);
        assert!(parsed.is_none());
    }
}
