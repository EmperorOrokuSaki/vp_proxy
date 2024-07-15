use std::cell::RefCell;

use ic_exports::ic_kit::Principal;
use ic_sns_governance::pb::v1::NeuronId;

use crate::{types::{CanisterError, CouncilMember}, utils::not_anonymous};

thread_local! {
    pub static COUNCIL_MEMBERS: RefCell<Vec<CouncilMember>> = RefCell::new(Vec::new());
    pub static GOVERNANCE_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_governance(id: Principal)
    pub static LEDGER_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_ledger(id: Principal)

    pub static NEURON_ID: RefCell<Option<NeuronId>> = RefCell::new(None);
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