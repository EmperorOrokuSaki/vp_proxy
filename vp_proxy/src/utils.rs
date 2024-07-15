use ic_exports::{ic_cdk::api::is_controller, ic_kit::Principal};

use crate::{state::GOVERNANCE_CANISTER_ID, types::CanisterError};

pub fn only_controller(caller: Principal) -> Result<(), CanisterError> {
    if !is_controller(&caller) {
        // only the controller (the DAO) should be able to call this function
        return Err(CanisterError::Unauthorized);
    }
    Ok(())
}

pub fn is_governance_set() -> Result<(), CanisterError> {
    GOVERNANCE_CANISTER_ID.with(|id| {
        if id.borrow() == Principal::anonymous() {
            return Err(CanisterError::GovernanceCanisterIdNotSet);
        }
        Ok(())
    })
}
