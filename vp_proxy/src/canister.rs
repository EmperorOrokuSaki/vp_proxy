use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::{
    candid::{Nat, Principal},
    ic_cdk::{call, caller, id},
    ic_kit::{ic::time, CallResult},
};
use ic_nervous_system_common::ledger;
use ic_sns_governance::pb::v1::{
    manage_neuron::{
        self,
        claim_or_refresh::{By, MemoAndController},
        ClaimOrRefresh,
    },
    ManageNeuron, ManageNeuronResponse, NeuronId,
};
use icrc_ledger_types::icrc1::{
    account::Account,
    transfer::{TransferArg, TransferError},
};

use crate::{
    state::{
        get_governance_canister_id, get_ledger_canister_id, COUNCIL_MEMBERS,
        GOVERNANCE_CANISTER_ID, LEDGER_CANISTER_ID, NEURON_ID,
    },
    types::{CanisterError, CouncilMember},
    utils::{handle_intercanister_call, only_controller},
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
    pub async fn create_neuron(&self, amount: Nat, nonce: u64) -> Result<NeuronId, CanisterError> {
        only_controller(caller())?;
        // transfers all CONF tokens to the neuron's subaccount under the governance canister id
        let subaccount = ledger::compute_neuron_staking_subaccount(id(), nonce);
        let governance_canister_id = get_governance_canister_id()?;
        let ledger_canister_id = get_ledger_canister_id()?;

        let transfer_args = TransferArg {
            from_subaccount: None,
            to: Account {
                owner: governance_canister_id,
                subaccount: Some(subaccount.0),
            },
            fee: None,
            created_at_time: None,
            memo: nonce,
            amount,
        };

        let transfer_response = call(ledger_canister_id, "icrc1_transfer", (transfer_args,)).await;

        match handle_intercanister_call::<Result<Nat, TransferError>>(transfer_response)? {
            Err(err) => Err(CanisterError::Unknown(format!(
                "Error occured on token transfer: {:#?}",
                err
            ))),
            _ => {}
        }

        // claim neuron
        let neuron_claim_args = ManageNeuron {
            subaccount: subaccount.to_vec(),
            command: Some(manage_neuron::Command::ClaimOrRefresh(ClaimOrRefresh {
                by: Some(By::MemoAndController(MemoAndController {
                    memo: nonce,
                    controller: Some(id()),
                })),
            })),
        };

        let claim_response = call(
            governance_canister_id,
            "manage_neuron",
            (neuron_claim_args,),
        )
        .await;

        let manage_neuron_response =
            handle_intercanister_call::<ManageNeuronResponse>(claim_response)?;

        if let Some(command) = manage_neuron_response.command {
            let neuron_id = match command {
                ic_sns_governance::pb::v1::manage_neuron_response::Command::ClaimOrRefresh(
                    claim_or_refresh_response,
                ) => Ok(claim_or_refresh_response.refreshed_neuron_id),
                _ => Err(CanisterError::Unknown(
                    "Could not handle the manage neuron response".to_string(),
                )),
            }?;

            if let Some(neuron_id_unwrapped) = neuron_id {
                NEURON_ID.with(|id| *id.borrow_mut() = Some(neuron_id_unwrapped.clone()));
                return Ok(neuron_id_unwrapped);
            }

            return Err(CanisterError::Unknown("Neuron Id couldn't be generated."));
        } else {
            return Err(CanisterError::Unknown(
                "Could not handle the manage neuron response".to_string(),
            ));
        }
    }

    #[update]
    pub fn add_council_member(
        &self,
        name: String,
        neuron_id: NeuronId,
    ) -> Result<(), CanisterError> {
        only_controller(caller())?;
        COUNCIL_MEMBERS
            .with(|members| members.borrow_mut().push(CouncilMember { name, neuron_id }));
        Ok(())
    }

    #[update]
    pub fn remove_council_member(&self, neuron_id: NeuronId) -> Result<(), CanisterError> {
        only_controller(caller())?;
        COUNCIL_MEMBERS
            .with(|members| members.borrow_mut().retain(|member| member.neuron_id != neuron_id));
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

    #[update]
    pub fn start_timers(&self) -> Result<(), CanisterError> {
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
