use ic_exports::{candid::CandidType, ic_cdk_timers::TimerId};
use ic_sns_governance::pb::v1::ProposalId;

#[derive(Clone, CandidType)]
pub struct ProxyProposalQuery {
    pub id: ProposalId,
    pub action: u64,
    pub creation_timestamp: u64,
}

#[derive(Clone)]
pub struct ProxyProposal {
    pub id: ProposalId,
    pub action: u64,
    pub creation_timestamp: u64,
    pub timer_id: Option<TimerId>,
}

impl From<ProxyProposal> for ProxyProposalQuery {
    fn from(value: ProxyProposal) -> Self {
        Self {
            id: value.id,
            action: value.action,
            creation_timestamp: value.creation_timestamp,
        }
    }
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
