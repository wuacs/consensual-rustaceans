use crate::types::*;
use crate::proposer::*;

pub enum PaxosMsg<V> {
    Prepare { proposal_id: ProposalId, from: NodeId },
    Promise { accepted_proposal: Option<Proposal<V>>, proposal_response: ProposalId},
    AcceptRequest { node_id: NodeId, v: V },
    AcceptedProposal { node_id: u64, proposal_id: u64, value: V },
}