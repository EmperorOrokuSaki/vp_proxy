use std::time::Duration;

use ic_canister::{generate_idl, query, update, Canister, Idl, PreUpdate};
use ic_exports::{
    candid::{Nat, Principal},
    ic_cdk::{call, caller, id, print, spawn},
    ic_cdk_timers::{clear_timer, set_timer, set_timer_interval},
};
use ic_nervous_system_common::ledger;
use ic_sns_governance::pb::v1::{
    manage_neuron::{
        self,
        claim_or_refresh::{By, MemoAndController},
        ClaimOrRefresh,
    },
    ManageNeuron, ManageNeuronResponse, NeuronId, ProposalId,
};
use icrc_ledger_types::icrc1::{
    account::Account,
    transfer::{Memo, TransferArg, TransferError},
};

use crate::{
    proposals::check_proposals,
    state::{
        get_governance_canister_id, get_ledger_canister_id, get_max_retries, get_proposal_history,
        get_proposal_watchlist, COUNCIL_MEMBERS, EXCLUDED_ACTION_IDS, GOVERNANCE_CANISTER_ID,
        LAST_PROPOSAL, NEURON_ID, WATCHING_PROPOSALS,
    },
    types::{CanisterError, CouncilMember, ProposalHistory, ProxyProposal, ProxyProposalQuery},
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
        let subaccount = ledger::compute_neuron_staking_subaccount(id().into(), nonce);
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
            memo: Some(Memo::from(nonce)),
            amount,
        };

        let transfer_response = call(ledger_canister_id, "icrc1_transfer", (transfer_args,)).await;

        match handle_intercanister_call::<Result<Nat, TransferError>>(transfer_response)? {
            Err(err) => Err(CanisterError::Unknown(format!(
                "Error occured on token transfer: {:#?}",
                err
            ))),
            _ => Ok(()),
        }?;

        // claim neuron
        let neuron_claim_args = ManageNeuron {
            subaccount: subaccount.to_vec(),
            command: Some(manage_neuron::Command::ClaimOrRefresh(ClaimOrRefresh {
                by: Some(By::MemoAndController(MemoAndController {
                    memo: nonce,
                    controller: Some(id().into()),
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

            return Err(CanisterError::Unknown(
                "Neuron Id couldn't be generated.".to_string(),
            ));
        } else {
            return Err(CanisterError::Unknown(
                "Could not handle the manage neuron response".to_string(),
            ));
        }
    }

    #[update]
    pub fn add_council_member(&self, name: String, neuron_id: String) -> Result<(), CanisterError> {
        only_controller(caller())?;
        COUNCIL_MEMBERS
            .with(|members| members.borrow_mut().push(CouncilMember { name, neuron_id }));
        Ok(())
    }

    #[update]
    pub fn remove_council_member(&self, neuron_id: String) -> Result<(), CanisterError> {
        only_controller(caller())?;
        COUNCIL_MEMBERS.with(|members| {
            members
                .borrow_mut()
                .retain(|member| member.neuron_id != neuron_id)
        });
        Ok(())
    }

    #[update]
    pub fn emergency_reset(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;
        COUNCIL_MEMBERS.with(|members| *members.borrow_mut() = vec![]); // any timer should be cancelled?
        Ok(())
    }

    #[update]
    pub fn allow_action_type(&self, action_type: u64) -> Result<(), CanisterError> {
        only_controller(caller())?;
        EXCLUDED_ACTION_IDS
            .with(|actions| actions.borrow_mut().retain(|action| action != &action_type));
        Ok(())
    }

    #[update]
    pub fn disallow_action_type(&self, action_type: u64) -> Result<(), CanisterError> {
        only_controller(caller())?;
        EXCLUDED_ACTION_IDS.with(|actions| actions.borrow_mut().push(action_type));
        WATCHING_PROPOSALS.with(|proposals| {
            let mut proposals_mutable = proposals.borrow_mut();
            proposals_mutable.iter().for_each(|proposal| {
                if proposal.action == action_type && proposal.timer_id.is_some() {
                    // cancel its timer
                    clear_timer(proposal.timer_id.unwrap());
                }
            });
            proposals_mutable.retain(|proposal| proposal.action != action_type);
        });
        Ok(())
    }

    #[update]
    pub fn watch_proposals(
        &self,
        from_proposal: ProposalId,
        from_proposal_action: u64,
        from_proposal_creation_timestamp: u64,
    ) -> Result<(), CanisterError> {
        only_controller(caller())?;
        LAST_PROPOSAL.with(|proposal| {
            *proposal.borrow_mut() = Some(ProxyProposal {
                id: from_proposal,
                action: from_proposal_action,
                creation_timestamp: from_proposal_creation_timestamp,
                timer_id: None,
            })
        });

        set_timer(Duration::ZERO, || {
            spawn(async {
                let max_retries = get_max_retries();
                for _ in 0..max_retries {
                    let checked_proposals = check_proposals().await;
                    if checked_proposals.is_err() {
                        let err = checked_proposals.err().unwrap();
                        print(format!(
                            "Proposals check cycle failed. Retrying. Returned error is: {:#?}",
                            err
                        ));
                    } else {
                        break;
                    }
                }
            })
        });

        set_timer_interval(Duration::from_secs(86_400), || {
            spawn(async {
                loop {
                    let checked_proposals = check_proposals().await;
                    if checked_proposals.is_err() {
                        let err = checked_proposals.err().unwrap();
                        print(format!(
                            "Proposals check cycle failed. Retrying. Returned error is: {:#?}",
                            err
                        ));
                    } else {
                        break;
                    }
                }
            })
        });

        Ok(())
    }

    #[query]
    pub fn get_council(&self) -> Vec<CouncilMember> {
        COUNCIL_MEMBERS.with(|members| members.borrow().clone())
    }

    #[query]
    pub fn get_proposal_history(&self) -> Vec<ProposalHistory> {
        get_proposal_history()
    }

    #[query]
    pub fn get_proposal_watchlist(&self) -> Vec<ProxyProposalQuery> {
        get_proposal_watchlist()
    }

    pub fn idl() -> Idl {
        generate_idl!()
    }
}
