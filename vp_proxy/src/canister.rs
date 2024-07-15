use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::{
    candid::{Nat, Principal},
    ic_cdk::{call, caller, id},
    ic_kit::ic::time,
};
use ic_nervous_system_common::ledger;
use icrc_ledger_types::icrc1::{account::Account, transfer::TransferArg};

use crate::{
    state::{
        get_governance_canister_id, get_ledger_canister_id, COUNCIL_MEMBERS,
        GOVERNANCE_CANISTER_ID, LEDGER_CANISTER_ID,
    },
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
    pub async fn create_neuron(&self, amount: Nat, nonce: u64) -> Result<(), CanisterError> {
        only_controller(caller())?;
        // transfers all CONF tokens to the neuron's subaccount under the governance canister id
        let subaccount = ledger::compute_neuron_staking_subaccount(id(), nonce);
        let governance_canister_id = get_governance_canister_id()?;
        let ledger_canister_id = get_ledger_canister_id()?;

        let transfer_args = TransferArg {
            from_subaccount: None,
            to: Account {
                owner: governance_canister_id,
                subaccount: Some(subaccount.0)
            },
            fee: None,
            created_at_time: None,
            memo: nonce,
            amount,
        };

        match call(ledger_canister_id, ("icrc1_transfer",), (transfer_args,)).await {

        }

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
        COUNCIL_MEMBERS.with(|members| *members.borrow_mut() = vec![]); // any timer should be cancelled?
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
