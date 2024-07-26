use ic_exports::{
    candid::{CandidType, Principal},
    ic_cdk_timers::TimerId,
};
use ic_sns_governance::pb::v1::{NeuronId, ProposalId};

#[derive(Clone)]
pub struct ProxyProposal {
    pub id: ProposalId,
    pub action: u64,
    pub creation_timestamp: u64,
    pub timer_id: Option<TimerId>,
}

#[derive(CandidType, Clone)]
pub struct CouncilMember {
    pub name: String,
    pub neuron_id: String,
}

#[derive(CandidType, Debug)]
pub enum CanisterError {
    Unknown(String),
    Unauthorized,
    ConfigurationError,
}

#[derive(CandidType, Clone)]
pub struct ProposalHistory {
    pub proposal_id: ProposalId,
    pub participation_status: ParticipationStatus,
}

#[derive(CandidType, Clone)]
pub enum ParticipationStatus {
    Abstained,
    VotedFor,
    VotedAgainst,
}
