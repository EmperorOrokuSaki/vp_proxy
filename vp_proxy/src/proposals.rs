use std::time::Duration;

use ic_exports::{
    ic_cdk::{api::time, call, print, spawn},
    ic_cdk_timers::set_timer,
};
use ic_sns_governance::pb::v1::{GetProposal, GetProposalResponse, ListProposals, ListProposalsResponse, ProposalId};

use crate::{
    state::{
        get_governance_canister_id, get_last_proposal_id, get_max_retries, EXCLUDED_ACTION_IDS, GOVERNANCE_CANISTER_ID, LAST_PROPOSAL, WATCHING_PROPOSALS
    },
    types::{CanisterError, LastProposal},
    utils::handle_intercanister_call,
};

pub async fn check_proposals() -> Result<(), CanisterError> {
    let last_proposal = get_last_proposal_id()?;
    let governance_canister_id = get_governance_canister_id()?;

    let excluded_actions = EXCLUDED_ACTION_IDS.with(|vector| vector.borrow().clone());
    let mut before_proposal: Option<ProposalId> = None;
    loop {
        let list_proposals_arg = ListProposals {
            limit: 100,
            before_proposal,
            exclude_type: excluded_actions,
            include_reward_status: vec![],
            include_status: vec![],
        };

        let get_proposals_response = call(
            governance_canister_id,
            "list_proposals",
            (list_proposals_arg,),
        )
        .await;

        let list_proposals_handled =
            handle_intercanister_call::<ListProposalsResponse>(get_proposals_response)?;

        list_proposals_handled.proposals.iter().map(|proposal| {
            if proposal.id.unwrap() == last_proposal.id
                || proposal.proposal_creation_timestamp_seconds <= last_proposal.creation_timestamp
            {
                LAST_PROPOSAL.with(|proposal| {
                    *proposal.borrow_mut() = Some(LastProposal {
                        id: list_proposals_handled.proposals[0].id,
                        action: proposal.action,
                        creation_timestamp: list_proposals_handled.proposals[0]
                            .proposal_creation_timestamp_seconds,
                    });
                });
                before_proposal = None;
            } else {
                WATCHING_PROPOSALS
                    .with(|proposals| proposals.borrow_mut().push(proposal.id.unwrap()));
                let current_time = time() / 1_000_000_000;
                let deadline = proposal.initial_voting_period_seconds
                    + proposal.proposal_creation_timestamp_seconds
                    - 3600;
                let remaining_time = deadline - current_time;
                set_timer(
                    Duration::from_secs(remaining_time),
                    spawn(|| async {
                        let max_retries = get_max_retries();
                        for _ in 0..max_retries {
                            let checked_proposals = vote_on_proposal(proposal.id).await;
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
                    }),
                );
                before_proposal = proposal.id;
            }
        });

        if before_proposal.is_none() {
            break;
        }
    }
}

pub async fn vote_on_proposal(id: ProposalId) -> Result<(), CanisterError> {
    // time to vote on the proposal
    let governance_canister_id = get_governance_canister_id()?;
    
    let get_proposal_arg = GetProposal {
        proposal_id: Some(id),
    };

    let get_proposal_response = call(
        governance_canister_id,
        "get_proposal",
        (get_proposal_arg,),
    )
    .await;

    let get_proposal_handled =
        handle_intercanister_call::<GetProposalResponse>(get_proposal_response)?;
    
    if get_proposal_handled.result.is_none() {
        return Err(CanisterError::Unknown(format!("Proposal could not be found. Id: {:#?}", id)));
    }

    

    Ok(())
}
