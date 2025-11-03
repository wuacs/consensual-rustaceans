use crate::types::*;
use crate::proposer::*;

pub enum PaxosMsg<V> {
    Prepare { proposal_id: ProposalId, from: NodeId },
    Promise { accepted_proposal: Option<Proposal<V>>, proposal_response: ProposalId},
    AcceptProposal { proposal_id: ProposalId, value: V },
    Accepted { proposal: Proposal<V> },
    Learn { proposal_id: ProposalId, value: V}
}