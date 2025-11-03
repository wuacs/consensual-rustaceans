use crate::{types::*, msg::PaxosMsg};
use std::collections::HashSet;

#[derive(Clone)]
pub struct Proposal<V> {
    pub id: ProposalId,
    pub value: V,
}

// Eq/Ord/Hash by id
impl<V> PartialEq for Proposal<V> { fn eq(&self, o: &Self) -> bool { self.id == o.id } }
impl<V> Eq for Proposal<V> {}
impl<V> std::hash::Hash for Proposal<V> { fn hash<H: std::hash::Hasher>(&self, s: &mut H) { self.id.hash(s); } }
impl<V> PartialOrd for Proposal<V> { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.id.cmp(&o.id)) } }
impl<V> Ord for Proposal<V> { fn cmp(&self, o: &Self) -> std::cmp::Ordering { self.id.cmp(&o.id) } }

/// Single, compact state for the current proposer round.
struct RoundState<V> {
    proposal_id: ProposalId,
    // Prepare step
    promises_from: HashSet<NodeId>,
    highest_accepted: Option<Proposal<V>>,
    // Accept step
    accept_acks: HashSet<NodeId>,
}

impl<V> RoundState<V> {
    fn new(proposal_id: ProposalId) -> Self {
        Self {
            proposal_id,
            promises_from: HashSet::new(),
            highest_accepted: None,
            accept_acks: HashSet::new(),
        }
    }
}

pub struct Proposer<V> {
    node_id: NodeId,
    ctx: NodeContext,
    peers: Vec<NodeId>,           // who we talk to (acceptors/quorum)
    next_pid: ProposalId,
    candidate_value: V,
    quorum: usize,
    round: Option<RoundState<V>>,
    timer_id: TimerId,
    timer_ms: u64,
}

impl<V: Clone> Proposer<V> {
    pub fn new(node_id: NodeId, ctx: NodeContext, peers: Vec<NodeId>, candidate_value: V, timer_ms: u64, quorum: usize) -> Self {
        Self {
            node_id,
            ctx,
            peers,
            quorum,
            next_pid: (0, node_id),
            candidate_value,
            round: None,
            timer_id: (0, node_id),
            timer_ms,
        }
    }

    fn quorum(&self) -> usize {
        (self.ctx.number_of_nodes / 2 + 1) as usize
    }

    fn next_proposal_id(&mut self) -> ProposalId {
        let pid = self.next_pid;
        self.next_pid.0 = self.next_pid.0.saturating_add(1);
        pid
    }

    fn next_timer_id(&mut self) -> TimerId {
        let tid = self.timer_id;
        self.timer_id.0 = self.timer_id.0.saturating_add(1);
        tid
    }

    fn start_round(&mut self) -> Vec<Action<V>> {
        let pid = self.next_proposal_id();
        self.round = Some(RoundState::new(pid));
        let tid = self.next_timer_id();

        let mut actions: Vec<Action<V>> = self.broadcast_prepare(pid);
        actions.push(Action::SetTimer { id: tid, ms: self.timer_ms });
        actions
    }

    fn broadcast_prepare(&self, pid: ProposalId) -> Vec<Action<V>> {
        self.peers.iter().copied().map(|to| Action::Send {
            to,
            from: self.node_id,
            msg: PaxosMsg::Prepare { proposal_id: pid, from: self.node_id },
        }).collect()
    }

    fn broadcast_accept(&self, pid: ProposalId, v: V) -> Vec<Action<V>> {
        self.peers.iter().copied().map(|to| Action::Send {
            to,
            from: self.node_id,
            msg: PaxosMsg::AcceptProposal { proposal_id: pid, value: v.clone() },
        }).collect()
    }

    pub fn on_init(&mut self) -> Vec<Action<V>> {
        self.start_round()
    }

    pub fn on_message(&mut self, from: NodeId, msg: PaxosMsg<V>) -> Vec<Action<V>> {
        match msg {
            PaxosMsg::Promise { accepted_proposal, proposal_response } => {
                let q = self.quorum; // take from &self BEFORE mutable borrow

                // Do all mutations on the round in a short scope
                let maybe_send: Option<(ProposalId, V)> = {
                    let r = match self.round.as_mut() {
                        Some(r) => r,
                        None => return vec![],
                    };
                    if r.proposal_id != proposal_response { return vec![]; }
                    if !r.promises_from.insert(from) { return vec![]; }

                    if let Some(p) = accepted_proposal {
                        if r.highest_accepted.as_ref().map_or(true, |best| p.id > best.id) {
                            r.highest_accepted = Some(p);
                        }
                    }

                    if r.promises_from.len() >= q {
                        let v = r.highest_accepted
                            .as_ref()
                            .map(|p| p.value.clone())
                            .unwrap_or_else(|| self.candidate_value.clone());
                        Some((r.proposal_id, v))
                    } else {
                        None
                    }
                };
                if let Some((pid, v)) = maybe_send {
                    return self.broadcast_accept(pid, v);
                }
                vec![]
            },
            _ => vec![]
        }
    }

    pub fn on_timeout(&mut self, id: TimerId) -> Vec<Action<V>> {
        if self.timer_id != id { return vec![]; } // stale
        self.timer_ms = self.timer_ms.saturating_mul(2);
        // Restart round with a higher proposal id
        self.start_round()
    }
}


/* If you use the trait abstraction */
impl<V: Clone> HandlesEvents<V> for Proposer<V> {
    fn on_init(&mut self) -> Vec<Action<V>> { self.on_init() }
    fn on_message(&mut self, from: NodeId, msg: PaxosMsg<V>) -> Vec<Action<V>> { self.on_message(from, msg) }
    fn on_timeout(&mut self, id: TimerId) -> Vec<Action<V>> { self.on_timeout(id) }
}
