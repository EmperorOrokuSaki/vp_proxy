use ic_exports::candid::{CandidType, Principal};

#[derive(CandidType)]
pub struct CouncilMember {
    name: String,
    principal_id: Principal
}

#[derive(CandidType)]
pub enum CanisterError {
    Unknown(String),
    Unauthorized,
    GovernanceCanisterIdNotSet,
}