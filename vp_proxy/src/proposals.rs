use ic_exports::ic_cdk::call;
use ic_sns_governance::pb::v1::{ListProposals, ListProposalsResponse, ProposalId};

use crate::{
    state::{
        get_governance_canister_id, get_last_proposal_id, EXCLUDED_ACTION_IDS,
        GOVERNANCE_CANISTER_ID, LAST_PROPOSAL, WATCHING_PROPOSALS,
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
                        creation_timestamp: list_proposals_handled.proposals[0]
                            .proposal_creation_timestamp_seconds,
                    });
                });
                before_proposal = None;
            } else {
                WATCHING_PROPOSALS
                    .with(|proposals| proposals.borrow_mut().push(proposal.id.unwrap()));
                before_proposal = proposal.id;
            }
        });

        if before_proposal.is_none() {
            break;
        }
    }
}
