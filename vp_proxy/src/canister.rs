use std::time::Duration;

use ic_canister::{
    generate_idl, post_upgrade, pre_upgrade, query, update, Canister, Idl, PreUpdate,
};
use ic_exports::{
    candid::{Nat, Principal},
    ic_cdk::{call, caller, id, print, spawn, storage},
    ic_cdk_timers::{clear_timer, set_timer, set_timer_interval},
};
use ic_nervous_system_common::ledger;
use ic_sns_governance::pb::v1::{
    manage_neuron::{
        self,
        claim_or_refresh::{By, MemoAndController},
        configure::Operation,
        ClaimOrRefresh, Configure, IncreaseDissolveDelay,
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
        get_council_members, get_exclusion_list, get_fetcher_timer_id, get_governance_canister_id,
        get_ledger_canister_id, get_max_retries, get_neuron, get_proposal_history,
        get_proposal_watchlist, get_watch_lock, COUNCIL_MEMBERS, EXCLUDED_ACTION_IDS,
        FETCHER_TIMER_ID, GOVERNANCE_CANISTER_ID, LAST_PROPOSAL, LEDGER_CANISTER_ID, NEURON_ID,
        PROPOSAL_HISTORY, WATCHING_PROPOSALS, WATCH_LOCK,
    },
    types::{CanisterError, CouncilMember, ParticipationStatus, ProxyProposalQuery},
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
    pub fn set_ledger_id(&self, canister_id: Principal) -> Result<(), CanisterError> {
        only_controller(caller())?;
        LEDGER_CANISTER_ID.with(|id| *id.borrow_mut() = canister_id);
        Ok(())
    }

    #[update]
    pub async fn create_neuron(&self, amount: Nat, nonce: u64) -> Result<NeuronId, CanisterError> {
        only_controller(caller())?;

        if get_neuron().is_ok() {
            return Err(CanisterError::NeuronAlreadySet);
        }

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
    pub async fn increase_disolve_delay(&self, delay: u32) -> Result<(), CanisterError> {
        only_controller(caller())?;

        let neuron_id = get_neuron()?;
        let governance_canister_id = get_governance_canister_id()?;

        let neuron_claim_args = ManageNeuron {
            subaccount: neuron_id.id,
            command: Some(manage_neuron::Command::Configure(Configure {
                operation: Some(Operation::IncreaseDissolveDelay(IncreaseDissolveDelay {
                    additional_dissolve_delay_seconds: delay,
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
            return match command {
                ic_sns_governance::pb::v1::manage_neuron_response::Command::Configure(_) => Ok(()),
                _ => Err(CanisterError::Unknown(
                    "Could not handle the manage neuron response".to_string(),
                )),
            };
        }
        return Err(CanisterError::Unknown(
            "Could not handle the manage neuron response".to_string(),
        ));
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
    pub fn stop_timers(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;

        if !get_watch_lock() {
            // lock is off.
            return Err(CanisterError::WatchingIsAlreadyStopped);
        }

        // Cancel all timers
        WATCHING_PROPOSALS.with(|proposals| {
            let mut proposals = proposals.borrow_mut();
            for proposal in proposals.iter_mut() {
                proposal.lock = true;
                if let Some(timer_id) = proposal.timer_id {
                    clear_timer(timer_id);
                }
                proposal.lock = false;
            }
            *proposals = vec![];
        });

        let fetcher_timer_id = get_fetcher_timer_id();

        if fetcher_timer_id.is_some() {
            clear_timer(fetcher_timer_id.unwrap());
        }

        FETCHER_TIMER_ID.with(|id| *id.borrow_mut() = None);

        WATCH_LOCK.with(|lock| lock.set(false));

        Ok(())
    }

    #[update]
    pub fn clear_proposal_history(&self) -> Result<(), CanisterError> {
        only_controller(caller())?;
        PROPOSAL_HISTORY.with(|history| *history.borrow_mut() = vec![]);
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
        get_neuron()?;
        get_governance_canister_id()?;
        get_ledger_canister_id()?;

        if get_watch_lock() {
            // lock is already turned on.
            return Err(CanisterError::WatchingIsAlreadyInProgress);
        }

        LAST_PROPOSAL.with(|proposal| {
            *proposal.borrow_mut() = Some(ProxyProposalQuery {
                id: from_proposal,
                action: from_proposal_action,
                creation_timestamp: from_proposal_creation_timestamp,
                participation_status: ParticipationStatus::Undecided, // doesn't matter
                timer_scheduled_for: None,
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

        WATCH_LOCK.with(|lock| lock.set(true));

        Ok(())
    }

    #[query]
    pub fn get_proposal_status(&self, id: ProposalId) -> Option<ProxyProposalQuery> {
        // check both history and watching proposals.
        let mut proposal: Option<ProxyProposalQuery> = None;
        PROPOSAL_HISTORY.with(|proposals| {
            let _ = proposals.borrow().iter().map(|proposal_data| {
                if proposal_data.id == id {
                    proposal = Some(proposal_data.clone().into())
                }
            });
        });

        WATCHING_PROPOSALS.with(|proposals| {
            let _ = proposals.borrow().iter().map(|proposal_data| {
                if proposal_data.id == id {
                    proposal = Some(proposal_data.clone().into())
                }
            });
        });

        proposal
    }

    #[query]
    pub fn get_council(&self) -> Vec<CouncilMember> {
        COUNCIL_MEMBERS.with(|members| members.borrow().clone())
    }

    #[query]
    pub fn get_proposal_history(&self) -> Vec<ProxyProposalQuery> {
        get_proposal_history()
    }

    #[query]
    pub fn get_proposal_watchlist(&self) -> Vec<ProxyProposalQuery> {
        get_proposal_watchlist()
    }

    #[query]
    pub fn get_exclusion_list(&self) -> Vec<u64> {
        get_exclusion_list()
    }

    #[query]
    pub fn get_neuron_id(&self) -> Result<NeuronId, CanisterError> {
        get_neuron()
    }

    #[query]
    pub fn get_governance_id(&self) -> Result<Principal, CanisterError> {
        get_governance_canister_id()
    }

    #[query]
    pub fn get_ledger_id(&self) -> Result<Principal, CanisterError> {
        get_ledger_canister_id()
    }

    #[query]
    pub fn get_watching_status(&self) -> bool {
        get_watch_lock()
    }

    #[pre_upgrade]
    fn pre_upgrade(&self) {
        let governance_canister_id = GOVERNANCE_CANISTER_ID.with(|id| id.borrow().clone());
        let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
        let council_members = get_council_members();
        let proposal_history = get_proposal_history();
        let excluded_action_ids = get_exclusion_list();
        let neuron_id = NEURON_ID.with(|id| id.borrow().clone());

        let _ = storage::stable_save((
            governance_canister_id,
            ledger_canister_id,
            council_members,
            proposal_history,
            excluded_action_ids,
            neuron_id,
        ));
    }

    #[post_upgrade]
    fn post_upgrade(&self) {
        let (
            governance_canister_id,
            ledger_canister_id,
            council_members,
            proposal_history,
            excluded_action_ids,
            neuron_id,
        ): (
            Principal,
            Principal,
            Vec<CouncilMember>,
            Vec<ProxyProposalQuery>,
            Vec<u64>,
            Option<NeuronId>,
        ) = storage::stable_restore().unwrap();

        GOVERNANCE_CANISTER_ID.with(|id| *id.borrow_mut() = governance_canister_id);
        LEDGER_CANISTER_ID.with(|id| *id.borrow_mut() = ledger_canister_id);

        COUNCIL_MEMBERS.with(|members| {
            let mut members_borrowed = members.borrow_mut();
            council_members
                .into_iter()
                .for_each(|member| members_borrowed.push(member));
        });

        PROPOSAL_HISTORY.with(|history| *history.borrow_mut() = proposal_history);
        EXCLUDED_ACTION_IDS.with(|ids| *ids.borrow_mut() = excluded_action_ids);
        NEURON_ID.with(|id| *id.borrow_mut() = neuron_id);
    }

    pub fn idl() -> Idl {
        generate_idl!()
    }
}
