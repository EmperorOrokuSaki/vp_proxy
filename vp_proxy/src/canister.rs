use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::{candid::Principal, ic_cdk::caller, ic_kit::ic::time};

use crate::{
    state::{COUNCIL_MEMBERS, GOVERNANCE_CANISTER_ID},
    types::{CanisterError, CouncilMember},
    utils::only_controller,
};

#[derive(Canister)]
pub struct VpProxy {
    #[id]
    id: Principal,
}

impl PreUpdate for VpProxy {}

impl VpProxy {
    #[update]
    pub fn set_governance_id(&self, canister_id: Principal) -> Result<(), CanisterError> {
        only_controller(caller())?;
        GOVERNANCE_CANISTER_ID.with(|id| *id.borrow_mut() = canister_id);
        Ok(())
    }

    #[update]
    pub fn create_neuron(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;
        Ok(())
    }

    #[update]
    pub fn add_council_member(&self, new_member: Principal) -> Result<(), CanisterError> {
        only_controller(caller())?;
        Ok(())
    }

    #[update]
    pub fn remove_council_member(&self, removed_member: Principal) -> Result<(), CanisterError> {
        only_controller(caller())?;
        Ok(())
    }

    #[update]
    pub fn emergency_reset(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;
        Ok(())
    }

    #[update]
    pub fn allow_proposal_type(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;
        Ok(())
    }

    #[update]
    pub fn disallow_proposal_type(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;
        Ok(())
    }

    #[query]
    pub fn get_council(&self) -> Vec<CouncilMember> {
        COUNCIL_MEMBERS.with(|members| members.borrow().clone())
    }

    pub fn idl() -> Idl {
        generate_idl!()
    }
}
