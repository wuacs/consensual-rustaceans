use crate::types::*;
use std::{collections::HashSet, hash::Hash, task::Context};
use crate::msg::PaxosMsg;

pub struct Proposal<V> {
    pub id: ProposalId,
    pub value: V,
}
impl<V> PartialEq for Proposal<V> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<V> Eq for Proposal<V> {}
impl<V> Hash for Proposal<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<V> PartialOrd for Proposal<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.cmp(&other.id))
    }
}
impl<V> Ord for Proposal<V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        return self.id.cmp(&other.id);
    }
}
/// Represents the state information of a Paxos Nodes actin as a Proposer in phase 1.
///  It keeps track of:
/// 1. "context": contextual information given by the model in use (e.g. known number of nodes)
/// 2. "proposal": the *current* "proposal" sent as part of the prepare message
/// 3. "node_responses": the set of nodes who have responded to the *current* "proposal" message
/// 4. "proposal_responses": the set of proposals received from nodes as part of their promise messages
pub struct ProposerPhase1State<V>{
    context: NodeContext,
    proposal: ProposalId,
    node_responses: HashSet<NodeId>,
    proposal_responses: HashSet<Proposal<V>>,
}
/// Represents the state information of a Paxos Nodes acting as a Proposer in phase 2.
/// It keeps track of:
/// 1. "context": contextual information given by the model in use (e.g. known number of nodes)
/// 2. "proposal": the *current* "proposal" sent as part of the accept message
/// 3. "accepted_nodes": the set of nodes who have responded with an accepted message to the *current* "proposal" message
pub struct ProposerPhase2State<V> {
    context: NodeContext,
    proposal: Proposal<V>,
    accepted_nodes: HashSet<NodeId>,
}

impl<V> ProposerPhase1State<V> where V: Clone {
    pub fn new(proposal_id: ProposalId, context: NodeContext) -> Self {
        Self {
            context: context,
            node_responses: HashSet::new(),
            proposal: proposal_id,
            proposal_responses: HashSet::new(),
        }
    }
    pub fn add_response(&mut self, node_id: NodeId, proposal_accepted: Option<Proposal<V>>, prepare_id: u64) -> Vec<Action<V>> {
        if (prepare_id != self.proposal) {
            return vec![];
        }
        match proposal_accepted {
            Some(p) => {
                self.proposal_responses.insert(Proposal {
                id: p.id,
                value: p.value.clone()});    
            }
            _ => {}
        }
        self.node_responses.insert(node_id);
        if self.node_responses.len() as u64 >= ((self.context.number_of_nodes / 2) + 1) {
            return self.proposal_responses.iter().max().map(|p| {
                vec![Action::ProposeValue { v: p.value.clone() }]
            }).unwrap_or_else(|| {
                vec![]
            });
        }
        return vec![];
    }
}
impl<V> ProposerPhase2State<V> where V: Clone {
    pub fn new(proposal: Proposal<V>, context: NodeContext) -> Self {
        Self {
            accepted_nodes: HashSet::new(),
            context: context,
            proposal: proposal,
        }
    }
    pub fn add_accepted(&mut self, node_id: NodeId, referring_proposal: ProposalId) -> Vec<Action<V>> {
        if self.accepted_nodes.contains(&node_id) || referring_proposal != self.proposal.id {
            return vec![];
        }
        self.accepted_nodes.insert(node_id);
        if self.accepted_nodes.len() as u64 >= (self.context.number_of_nodes / 2 + 1) {
            return vec![Action::ChoseValue { v: self.proposal.value.clone() }];
        } else {
            return vec![];
        }
    }
}
/// Represents a Paxos Proposer instance:
/// 1. "next_id": the next proposal identifier to be used
/// 2. "proposed_value": the value which the proposer is trying to get chosen
/// 3. "phase": the current phase of the proposer (Idle, Phase1, Phase2)
/// 4. "node_id": the identifier of the node acting as proposer
pub struct Proposer<V> {
    node_id: NodeId,
    node_context: NodeContext,
    majority: Vec<NodeId>,
    next_id: ProposalId,
    proposed_value: V,
    phase: Phase<V>,
    timer_id: TimerId,
    timer_duration_ms: u64,
}

impl<V: Clone> Proposer<V> {

    pub fn new(node_id: NodeId, 
        context: NodeContext,
        proposed_value: V,
        timer_initial_duration_ms: u64
    ) -> Self {
        Self {
            node_id: node_id,
            node_context: context,
            majority: vec![0; context.number_of_nodes as usize],
            next_id: 0,
            proposed_value: proposed_value,
            phase: Phase::Idle,
            timer_id: (0, node_id),
            timer_duration_ms: timer_initial_duration_ms,
        }
    }

    fn on_phase1_start(&mut self) -> Vec<Action<V>> {
        let n = self.next_proposal_id();
        let ctx = self.node_context; // assuming Copy (number_of_nodes + timeout)
        self.phase = Phase::Phase1(ProposerPhase1State::new(n, ctx));

        // (Re)start timer
        let tid = self.next_timer_id();
        self.timer_id = tid;

        // Broadcast PREPARE to quorum (or all — your choice)
        let mut actions = self.queue_to_majority(n);
        actions.push(Action::SetTimer { id: tid, ms: self.timer_duration_ms });
        actions
    }

    pub fn on_init(&mut self) -> Vec<Action<V>> {
        self.on_phase1_start()
    }

    fn queue_to_majority(&self, n: ProposalId) -> Vec<Action<V>> {
        self.majority
            .iter()
            .copied()
            .map(|to| Action::Send {
                to,
                msg: PaxosMsg::Prepare { proposal_id: n, from: self.node_id },
            })
            .collect()
    }
    
    /// Make a Proposer react on an event, producing a list of actions to be performed by the 
    /// scheduler (e.g., sending messages, setting timers, etc.).
    pub fn on_event(&mut self, e: Event<V>) -> Vec<Action<V>> {
        match e {
            Event::Message { from, msg } => self.on_message(from, msg),
            Event::Timeout { id }        => self.on_timeout(id),
        }
    }

    fn on_message(&mut self, node_id: NodeId, msg: PaxosMsg<V>) -> Vec<Action<V>> {
        match (&mut self.phase, msg) {
            (Phase::Phase1(p1), PaxosMsg::Promise { accepted_proposal, proposal_response }) => {
                p1.add_response(node_id, accepted_proposal, proposal_response);
                vec![]
            },
            // Messages that belong to the wrong phase/round are ignored
            _ => vec![],
        }
    }

    fn on_timeout(&mut self, id: TimerId) -> Vec<Action<V>> {
        // Ignore stale timers
        if self.timer_id != id { return vec![]; }

        match &mut self.phase {
            Phase::Phase1(_p1) => {
                // Exponential backoff (optional)
                self.timer_duration_ms = self.timer_duration_ms.saturating_mul(2);
                // Restart Phase 1 with next id
                self.on_phase1_start()
            }
            Phase::Phase2(_p2) => {
                // Conservative policy: fall back to Phase 1
                self.timer_duration_ms = self.timer_duration_ms.saturating_mul(2);
                self.on_phase1_start()
            }
            Phase::Idle => vec![], // nothing to do
        }
    }

    fn next_proposal_id(&mut self) -> ProposalId {
        let n = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        n
    }

    fn next_timer_id(&mut self) -> TimerId {
        let n = self.node_id;
        self.timer_id = (self.next_id, n);
        (n, self.node_id)
    }

}
enum Phase<V> {
    Idle,                  
    Phase1(ProposerPhase1State<V>),
    Phase2(ProposerPhase2State<V>),
}