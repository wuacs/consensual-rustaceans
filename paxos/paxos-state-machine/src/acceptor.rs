use std::collections::HashSet;

use crate::{
    msg::PaxosMsg,
    proposer::Proposal,
    types::*,
};

pub struct Acceptor<V> {
    context: NodeContext,
    node_id: NodeId,
    latest_accepted_proposal: Option<Proposal<V>>,
    latest_promise: Option<ProposalId>,
    learners: HashSet<NodeId>,
}

impl<V: Clone> Acceptor<V> {
    pub fn new(node_id: NodeId, context: NodeContext, learners: HashSet<NodeId>) -> Self {
        Self {
            node_id,
            context,
            latest_accepted_proposal: None,
            latest_promise: None,
            learners,
        }
    }

    /// Broadcast any message to all learners (like proposer’s queue_to_majority).
    /// Requires PaxosMsg<V>: Clone to reuse the message per recipient.
    fn learners_broadcast(&self, msg: PaxosMsg<V>) -> Vec<Action<V>>
    where
        PaxosMsg<V>: Clone,
    {
        self.learners
            .iter()
            .copied()
            .map(|to| Action::Send { to, from: self.node_id, msg: msg.clone() })
            .collect()
    }
}

impl<V: Clone> HandlesEvents<V> for Acceptor<V>
where
    PaxosMsg<V>: Clone, // for learners_broadcast
{
    fn on_init(&mut self) -> Vec<Action<V>> {
        vec![]
    }

    fn on_message(&mut self, from: NodeId, msg: PaxosMsg<V>) -> Vec<Action<V>> {
        match msg {
            // PREPARE: promise if proposal_id >= latest_promise
            PaxosMsg::Prepare { proposal_id, from: proposer } => {
                let can_promise = self
                    .latest_promise
                    .map_or(true, |p| proposal_id >= p);

                if can_promise {
                    self.latest_promise = Some(proposal_id);
                    return vec![Action::Send {
                        to: proposer,
                        from: self.node_id,
                        msg: PaxosMsg::Promise {
                            accepted_proposal: self.latest_accepted_proposal.clone(),
                            proposal_response: proposal_id,
                        },
                    }];
                }
                vec![]
            }

            // ACCEPT REQUEST: accept iff proposal_id >= latest_promise
            // NOTE: If your enum uses a different variant name/shape, adjust accordingly.
            PaxosMsg::AcceptProposal { proposal_id, value } => {
                let can_accept = self
                    .latest_promise
                    .map_or(true, |p| proposal_id >= p);

                if !can_accept {
                    return vec![]; // or NACK if you have one
                }

                // Record acceptance
                let accepted = Proposal { id: proposal_id, value: value.clone() };
                self.latest_promise = Some(proposal_id);
                self.latest_accepted_proposal = Some(accepted.clone());

                // Tell learners (shape depends on your PaxosMsg)
                // If you have a dedicated Learn/Chosen message, use that.
                // Example with a hypothetical Learn message:
                return self.learners_broadcast(PaxosMsg::Learn {
                    proposal_id,
                    value
                });

            }

            // Anything else is ignored by acceptor
            _ => vec![],
        }
    }

    fn on_timeout(&mut self, _id: TimerId) -> Vec<Action<V>> {
        // Acceptors typically do nothing on timeout.
        vec![]
    }
}