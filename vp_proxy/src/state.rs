use std::{cell::{Cell, RefCell}, collections::HashMap};

use ic_exports::ic_kit::Principal;
use ic_sns_governance::pb::v1::{NeuronId, ProposalId};

use crate::{
    types::{CanisterError, CouncilMember, LastProposal},
    utils::not_anonymous,
};

thread_local! {
    pub static GOVERNANCE_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_governance(id: Principal)
    pub static LEDGER_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_ledger(id: Principal)
    pub static MAX_RETRIES: Cell<u8> = Cell::new(3);

    pub static COUNCIL_MEMBERS: RefCell<Vec<CouncilMember>> = RefCell::new(Vec::new());
    pub static WATCHING_PROPOSALS: RefCell<Vec<ProposalId>> = RefCell::new(Vec::new());

    pub static EXCLUDED_ACTION_IDS: RefCell<Vec<u64>> = RefCell::new(Vec::new());

    pub static LAST_PROPOSAL: RefCell<Option<LastProposal>> = RefCell::new(None);
    pub static NEURON_ID: RefCell<Option<NeuronId>> = RefCell::new(None);
}

pub fn get_max_retries() -> u8 {
    MAX_RETRIES.with(|count| count.get())
}

pub fn get_governance_canister_id() -> Result<Principal, CanisterError> {
    let governance_canister_id = GOVERNANCE_CANISTER_ID.with(|id| id.borrow().clone());
    not_anonymous(&governance_canister_id)?;
    Ok(governance_canister_id)
}

pub fn get_ledger_canister_id() -> Result<Principal, CanisterError> {
    let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
    not_anonymous(&ledger_canister_id)?;
    Ok(ledger_canister_id)
}

pub fn get_last_proposal_id() -> Result<LastProposal, CanisterError> {
    let last_proposal_id = LAST_PROPOSAL.with(|id| id.borrow().clone());
    if last_proposal_id.is_some() {
        return Ok(last_proposal_id.unwrap());
    }
    Err(CanisterError::Unknown(
        "Undefined last proposal id".to_string(),
    ))
}
