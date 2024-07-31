use ic_exports::{candid::CandidType, ic_cdk_timers::TimerId};
use ic_sns_governance::pb::v1::ProposalId;
use serde::{Deserialize, Serialize};

#[derive(Clone, CandidType, Deserialize)]
pub struct ProxyProposalQuery {
    pub id: ProposalId,
    pub action: u64,
    pub creation_timestamp: u64,
    pub participation_status: ParticipationStatus,
}

#[derive(Clone)]
pub struct ProxyProposal {
    pub id: ProposalId,
    pub action: u64,
    pub creation_timestamp: u64,
    pub timer_id: Option<TimerId>,
    pub participation_status: ParticipationStatus,
    pub lock: bool,
}

impl From<ProxyProposalQuery> for ProxyProposal {
    fn from(value: ProxyProposalQuery) -> Self {
        Self {
            id: value.id,
            action: value.action,
            creation_timestamp: value.creation_timestamp,
            participation_status: value.participation_status,
            lock: false,
            timer_id: None,
        }
    }
}

impl From<ProxyProposal> for ProxyProposalQuery {
    fn from(value: ProxyProposal) -> Self {
        Self {
            id: value.id,
            action: value.action,
            creation_timestamp: value.creation_timestamp,
            participation_status: value.participation_status,
        }
    }
}

#[derive(CandidType, Clone, Serialize, Deserialize)]
pub struct CouncilMember {
    pub name: String,
    pub neuron_id: String,
}

#[derive(CandidType, Debug)]
pub enum CanisterError {
    Unknown(String),
    Unauthorized,
    ConfigurationError,
    NeuronAlreadySet,
    WatchingIsAlreadyInProgress,
    WatchingIsAlreadyStopped,
    ProposalIsNotInWatchlist,
    ProposalLocked,
}

#[derive(CandidType, Clone, Deserialize)]
pub enum ParticipationStatus {
    Undecided,
    Abstained,
    VotedFor,
    VotedAgainst,
}
