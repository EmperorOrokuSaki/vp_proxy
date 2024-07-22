use ic_exports::candid::{CandidType, Principal};
use ic_sns_governance::pb::v1::{NeuronId, ProposalId};

pub struct LastProposal {
    pub id: ProposalId,
    pub action: u64,
    pub creation_timestamp: u64
}

#[derive(CandidType)]
pub struct CouncilMember {
    pub name: String,
    pub neuron_id: NeuronId
}

#[derive(CandidType, Debug)]
pub enum CanisterError {
    Unknown(String),
    Unauthorized,
    ConfigurationError
}