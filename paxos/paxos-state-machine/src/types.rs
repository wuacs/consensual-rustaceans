use crate::msg::PaxosMsg;
pub type NodeId = u64;
pub type ProposalId = (u64, NodeId);
pub type TimerId = (u64, NodeId);

#[derive(Clone, Copy)]
pub struct NodeContext {
    pub number_of_nodes: u64,
}
pub struct AcceptorState<V> {
    highest_promise: Option<ProposalId>,
    accepted_id: Option<ProposalId>,
    accepted_value: Option<V>,
}
/// Represents the different phases of the Paxos protocol, these events
/// are fed to the state machine to trigger transitions.
/// The events defined are:
/// 1. Message: Represents an incoming Paxos message from another node.
/// 2. Timeout: Represents a timeout event, 
pub enum Event<V> {
    Message { from: NodeId, msg: PaxosMsg<V> },
    Timeout { id: TimerId  },
}
/// Generic event trait for Paxos roles that react to messages/timeouts.
pub trait HandlesEvents<V: Clone> {
    /// Optional hook to emit actions right after creation/activation.
    fn on_init(&mut self) -> Vec<Action<V>> { vec![] }
    /// Handle an inbound Paxos message.
    fn on_message(&mut self, from: NodeId, msg: PaxosMsg<V>) -> Vec<Action<V>>;
    /// Handle a timeout (default: ignore).
    fn on_timeout(&mut self, _id: TimerId) -> Vec<Action<V>> { vec![] }
    /// Unified dispatcher you can feed into your scheduler.
    fn on_event(&mut self, e: Event<V>) -> Vec<Action<V>> {
        match e {
            Event::Message { from, msg } => self.on_message(from, msg),
            Event::Timeout { id }        => self.on_timeout(id),
        }
    }
}
// ---------- Outputs from the core ----------
pub enum Action<V> {
    Send { to: NodeId, from: NodeId, msg: PaxosMsg<V> },
    SetTimer { id: TimerId, ms: u64 },
    CancelTimer { id: TimerId },
    ProposeValue { v: V },
    ChoseValue { v: V },
}