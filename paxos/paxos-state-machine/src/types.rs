use crate::msg::PaxosMsg;

pub type NodeId = u64;
pub type ProposalId = u64;
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

// ---------- Outputs from the core ----------
pub enum Action<V> {
    Send { to: NodeId, msg: PaxosMsg<V> },
    SetTimer { id: TimerId, ms: u64 },
    CancelTimer { id: TimerId },
    ProposeValue { v: V },
    ChoseValue { v: V },
}
