use ic_exports::ic_cdk::call;
use ic_sns_governance::pb::v1::{ListProposals, ListProposalsResponse};

use crate::{
    state::{
        get_governance_canister_id, get_last_proposal_id, GOVERNANCE_CANISTER_ID, LAST_PROPOSAL,
    },
    types::CanisterError,
    utils::handle_intercanister_call,
};

pub async fn check_proposals() -> Result<(), CanisterError> {
    // get all proposals since the last proposal
    // disregard the ones with the wrong types
    // disregard the ones that are already on the watchlist (and have a one-off timer set for one hour before their deadline) (this probably never happens because last proposal excludes them anyway)
    // add any new proposal that is remaining to the watch list (aka add one-off timers for them)
    let last_proposal = get_last_proposal_id()?;
    let governance_canister_id = get_governance_canister_id()?;

    let list_proposals_arg = ListProposals {
        limit: 100,
        before_proposal: Some(last_proposal),
        exclude_type: todo!(),
        include_reward_status: todo!(),
        include_status: todo!(),
    };

    let get_proposals_response = call(
        governance_canister_id,
        "list_proposals",
        (list_proposals_arg,),
    )
    .await;

    let list_proposals_handled =
        handle_intercanister_call::<ListProposalsResponse>(get_proposals_response)?;

    for proposal in list_proposals_handled.proposals {

    }

    if list_proposals_handled.proposals.len() > 1 {
        LAST_PROPOSAL.with(|proposal_id| *proposal_id.borrow_mut() = list_proposals_handled.proposals.last().unwrap().id);
    }

    if list_proposals_handled.proposals.len() == 100 {
        check_proposals().await?;
    }
}
