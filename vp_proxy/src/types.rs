use ic_exports::candid::{CandidType, Principal};
use ic_sns_governance::pb::v1::NeuronId;

#[derive(CandidType)]
pub struct CouncilMember {
    name: String,
    neuron_id: NeuronId
}

#[derive(CandidType, Debug)]
pub enum CanisterError {
    Unknown(String),
    Unauthorized,
    ConfigurationError
}