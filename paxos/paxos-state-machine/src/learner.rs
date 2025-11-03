// src/learner.rs
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use crate::{types::*, msg::PaxosMsg};
pub struct Learner<V> {
    node_id: NodeId,
    quorum: usize,
    acks: HashMap<ProposalId, HashSet<NodeId>>,
    chosen: HashMap<ProposalId, V>,
}
impl<V> Learner<V>
where
    V: Clone + Eq + Hash,
{
    pub fn new(node_id: NodeId, context: NodeContext) -> Self {
        let quorum = (context.number_of_nodes / 2 + 1) as usize;
        Self {
            node_id,
            quorum,
            acks: HashMap::new(),
            chosen: HashMap::new(),
        }
    }
    /// Optional: query if we already learned for a given proposal id.
    pub fn get_chosen(&self, pid: ProposalId) -> Option<&V> {
        self.chosen.get(&pid)
    }
    /// Record an Accepted from `from` for (pid, v). Returns Some(v) if this crosses quorum now.
    fn record_accepted(&mut self, from: NodeId, pid: ProposalId, v: V) -> Option<V> {
        // If we already chose for this pid, ignore further acks.
        if self.chosen.contains_key(&pid) {
            return None;
        }
        let entry = self.acks.entry(pid).or_insert_with(HashSet::new);
        // Deduplicate per-acceptor
        if !entry.insert(from) {
            return None;
        }
        if entry.len() >= self.quorum {
            // We just learned (pid, v)
            self.chosen.insert(pid, v.clone());
            // Optionally GC: drop other values tracked for this pid.
            self.acks.retain(|(seen_pid), _| *seen_pid != pid);
            return Some(v);
        }
        None
    }
}
/// Plug into your existing event trait.
impl<V> HandlesEvents<V> for Learner<V>
where
    V: Clone + Eq + Hash,
{
    fn on_init(&mut self) -> Vec<Action<V>> {
        vec![]
    }
    fn on_message(&mut self, from: NodeId, msg: PaxosMsg<V>) -> Vec<Action<V>> {
        match msg {
            // Acceptors notify learners when they accept a value.
            // Adjust the variant/fields to your PaxosMsg definition if they differ.
            PaxosMsg::Accepted { proposal } => {
                if let Some(chosen_v) = self.record_accepted(from, proposal.id, proposal.value.clone()) {
                    // Emit local "chosen" action. If you also want to announce to others,
                    // you can add Action::Send { to, msg: PaxosMsg::Chosen { ... } } here.
                    return vec![Action::ChoseValue { v: chosen_v }];
                }
                vec![]
            }
            _ => vec![],
        }
    }
    fn on_timeout(&mut self, _id: TimerId) -> Vec<Action<V>> {
        // Pure learner doesn’t use timers.
        vec![]
    }
}
