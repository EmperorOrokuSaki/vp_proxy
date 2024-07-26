use std::time::Duration;

use ic_exports::{
    ic_cdk::{api::time, call, print, spawn},
    ic_cdk_timers::set_timer,
};
use ic_sns_governance::pb::v1::{
    GetProposal, GetProposalResponse, ListProposals, ListProposalsResponse, ProposalData,
    ProposalId,
};

use crate::{
    state::{
        get_council_members, get_governance_canister_id, get_last_proposal_id, get_max_retries,
        EXCLUDED_ACTION_IDS, LAST_PROPOSAL, PROPOSAL_HISTORY, WATCHING_PROPOSALS,
    },
    types::{CanisterError, ParticipationStatus, ProposalHistory, ProxyProposal},
    utils::{handle_intercanister_call, vote},
};

pub async fn check_proposals() -> Result<(), CanisterError> {
    let last_proposal = get_last_proposal_id()?;
    let governance_canister_id = get_governance_canister_id()?;
    let excluded_actions = EXCLUDED_ACTION_IDS.with(|vector| vector.borrow().clone());
    let mut before_proposal: Option<ProposalId> = None;

    // we start a loop that continues until it reaches a point either before the last proposal indexed in the previous 24h cycle or the same proposal itself
    loop {
        let list_proposals_arg = ListProposals {
            limit: 100, // maximum limit set by dfinity's sns project
            before_proposal,
            exclude_type: excluded_actions.clone(),
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

        // goes through all proposals that were retrieved in this 100-item query
        for proposal in list_proposals_handled.proposals.iter() {
            if handle_proposal(
                proposal,
                &last_proposal,
                &mut before_proposal,
                &list_proposals_handled.proposals,
            )
            .await?
            {
                // the breaking point (behind the last proposal) has been reached, no need to continue further.
                break;
            }
        }

        if before_proposal.is_none() {
            // the breaking point (behind the last proposal) has been reached, no need to continue further.
            break Ok(());
        }
    }
}

async fn handle_proposal(
    proposal: &ProposalData,
    last_proposal: &ProxyProposal,
    before_proposal: &mut Option<ProposalId>,
    proposals: &Vec<ProposalData>,
) -> Result<bool, CanisterError> {
    if proposal.id.unwrap() == last_proposal.id
        || proposal.proposal_creation_timestamp_seconds <= last_proposal.creation_timestamp
    {
        LAST_PROPOSAL.with(|last_proposal_cell| {
            *last_proposal_cell.borrow_mut() = Some(ProxyProposal {
                id: proposals[0].id.unwrap(),
                action: proposals[0].action,
                creation_timestamp: proposals[0].proposal_creation_timestamp_seconds,
                timer_id: None,
            });
        });
        *before_proposal = None;
        Ok(true)
    } else {
        let current_time = time() / 1_000_000_000;
        let deadline = proposal.initial_voting_period_seconds
            + proposal.proposal_creation_timestamp_seconds
            - 3600;
        let remaining_time = deadline - current_time;

        let proposal_id = proposal.id.unwrap();
        let proposal_timer_id = set_timer(Duration::from_secs(remaining_time), move || {
            let proposal_id = proposal_id.clone();
            spawn(async move {
                let max_retries = get_max_retries();
                for _ in 0..max_retries {
                    let checked_proposals = vote_on_proposal(proposal_id).await;
                    if let Err(err) = checked_proposals {
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

        WATCHING_PROPOSALS.with(|proposals| {
            let proxy_proposal = ProxyProposal {
                id: proposal_id,
                action: proposal.action,
                creation_timestamp: proposal.proposal_creation_timestamp_seconds,
                timer_id: Some(proposal_timer_id),
            };
            proposals.borrow_mut().push(proxy_proposal);
        });

        *before_proposal = proposal.id;
        Ok(false)
    }
}

pub async fn vote_on_proposal(id: ProposalId) -> Result<(), CanisterError> {
    let governance_canister_id = get_governance_canister_id()?;

    let get_proposal_arg = GetProposal {
        proposal_id: Some(id),
    };

    let get_proposal_response =
        call(governance_canister_id, "get_proposal", (get_proposal_arg,)).await;

    let get_proposal_handled =
        handle_intercanister_call::<GetProposalResponse>(get_proposal_response)?;

    if get_proposal_handled.result.is_none() {
        return Err(CanisterError::Unknown(format!(
            "Proposal data could not be found. Id: {:#?}",
            id
        )));
    }

    let proposal_result = get_proposal_handled.result.unwrap();
    let mut participation_status = ParticipationStatus::Abstained;

    match proposal_result {
        ic_sns_governance::pb::v1::get_proposal_response::Result::Error(err) => {
            Err(CanisterError::Unknown(format!(
                "Governance error on proposal data: {}",
                err.error_message
            )))
        }
        ic_sns_governance::pb::v1::get_proposal_response::Result::Proposal(data) => {
            if data.decided_timestamp_seconds == 0 {
                // proposal is already decided
                return Ok(());
            }

            let council_members = get_council_members();
            let ballots = data.ballots;
            let mut decision: i32 = 0;
            let mut voters_count: u64 = 0;

            let voting_threshold: u64 = council_members.len() as u64 / 2;

            council_members.into_iter().for_each(|member| {
                let ballot = ballots.get(&member.neuron_id);
                if let Some(vote) = ballot {
                    // council member has voted.
                    decision += vote.vote;
                    voters_count += 1;
                }
            });

            if voters_count > voting_threshold {
                // more than 50% of council members have voted. Participate.
                if decision > 0 {
                    // vote yes
                    vote(id, 1).await?;
                    participation_status = ParticipationStatus::VotedFor;
                } else if decision < 0 || decision == 0 {
                    // vote no
                    vote(id, -1).await?;
                    participation_status = ParticipationStatus::VotedAgainst;
                }
            }

            // remove this proposal from the watchlist
            WATCHING_PROPOSALS
                .with(|proposals| proposals.borrow_mut().retain(|proposal| proposal.id != id));

            // add this proposal and the final decision of the canister to the history
            PROPOSAL_HISTORY.with(|proposals| {
                proposals.borrow_mut().push(ProposalHistory {
                    proposal_id: id,
                    participation_status,
                })
            });

            Ok(())
        }
    }
}
