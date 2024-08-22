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
        change_proposal_lock, get_council_members, get_governance_canister_id,
        get_last_proposal_id, get_max_retries, get_watch_lock, EXCLUDED_ACTION_IDS, LAST_PROPOSAL,
        PROPOSAL_HISTORY, WATCHING_PROPOSALS,
    },
    types::{CanisterError, ParticipationStatus, ProxyProposal, ProxyProposalQuery},
    utils::{handle_intercanister_call, vote},
};

pub async fn check_proposals() -> Result<(), CanisterError> {
    let last_proposal = get_last_proposal_id()?;
    let governance_canister_id = get_governance_canister_id()?;
    let excluded_actions = EXCLUDED_ACTION_IDS.with(|vector| vector.borrow().clone());
    let mut before_proposal: Option<ProposalId> = None;

    // we start a loop that continues until it reaches a point either before the last proposal indexed in the previous 24h cycle or the same proposal itself
    print("Starting the 24hs proposals check cycle.");
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
    last_proposal: &ProxyProposalQuery,
    before_proposal: &mut Option<ProposalId>,
    proposals: &Vec<ProposalData>,
) -> Result<bool, CanisterError> {
    if proposal.id.unwrap() == last_proposal.id
        || proposal.proposal_creation_timestamp_seconds <= last_proposal.creation_timestamp
    {
        LAST_PROPOSAL.with(|last_proposal_cell| {
            *last_proposal_cell.borrow_mut() = Some(ProxyProposalQuery {
                id: proposals[0].id.unwrap(),
                action: proposals[0].action,
                creation_timestamp: proposals[0].proposal_creation_timestamp_seconds,
                participation_status: ParticipationStatus::Undecided,
                timer_scheduled_for: None,
            });
        });
        *before_proposal = None;
        Ok(true)
    } else if proposal
        .proposal
        .as_ref()
        .unwrap()
        .title
        .starts_with("CONFIGURE COUNCIL NEURON")
    {
        // This is related to council neuron proxy configurations. Ignore.
        return Ok(false);
    } else if proposal.reward_event_end_timestamp_seconds.is_some() {
        return Ok(false);
    } else {
        let current_time = time() / 1_000_000_000;
        let deadline = proposal.initial_voting_period_seconds
            + proposal.proposal_creation_timestamp_seconds
            - 3600;

        if deadline <= current_time {
            return Ok(false);
        }

        let remaining_time = deadline - current_time;

        let proposal_id = proposal.id.unwrap();
        let proposal_action = proposal.action;
        let proposal_creation_timestamp = proposal.proposal_creation_timestamp_seconds;
        print(format!(
            "Scheduling vote on proposal id {} in {} seconds.",
            proposal_id.id, remaining_time
        ));
        let proposal_timer_id = set_timer(Duration::from_secs(remaining_time), move || {
            let proposal_id = proposal_id.clone();
            spawn(async move {
                let max_retries = get_max_retries();
                for attempt in 1..=max_retries {
                    let checked_proposal =
                        vote_on_proposal(proposal_id, proposal_action, proposal_creation_timestamp)
                            .await;
                    if let Err(err) = checked_proposal {
                        let _ = change_proposal_lock(proposal_id, false);
                        if attempt + 1 > max_retries {
                            print(format!(
                                "Voting failed for proposal id {}. Retry number {}. Returned error is: {:#?}. No more retries. Adding proposal to history with FailedToVote participation status.",
                                proposal_id.id,
                                attempt,
                                err
                            ));

                            // remove this proposal from the watchlist
                            WATCHING_PROPOSALS.with(|proposals| {
                                proposals
                                    .borrow_mut()
                                    .retain(|proposal| proposal.id != proposal_id)
                            });

                            // add this proposal and the final decision of the canister to the history
                            PROPOSAL_HISTORY.with(|proposals| {
                                proposals.borrow_mut().push(ProxyProposalQuery {
                                    id: proposal_id,
                                    action: proposal_action,
                                    creation_timestamp: proposal_creation_timestamp,
                                    participation_status: ParticipationStatus::FailedToVote,
                                    timer_scheduled_for: None,
                                });
                            });
                        } else {
                            print(format!(
                                "Voting failed for proposal id {}. Retry number {}. Returned error is: {:#?}. Retrying...",
                                proposal_id.id,
                                attempt,
                                err
                            ));
                        }
                    } else if let Ok(status) = checked_proposal {
                        print(format!(
                            "Voted successfully for proposal id {}. The final vote is: {:#?}",
                            proposal_id.id, status
                        ));
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
                participation_status: ParticipationStatus::Undecided,
                lock: false,
                timer_scheduled_for: Some(deadline),
            };
            proposals.borrow_mut().push(proxy_proposal);
        });

        *before_proposal = proposal.id;
        Ok(false)
    }
}

pub async fn vote_on_proposal(
    id: ProposalId,
    action: u64,
    creation_timestamp: u64,
) -> Result<ParticipationStatus, CanisterError> {
    if !get_watch_lock() {
        // lock is off.
        return Err(CanisterError::WatchingIsAlreadyStopped);
    }

    change_proposal_lock(id, true)?;

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
    let mut participation_status = ParticipationStatus::VotedAgainst;

    match proposal_result {
        ic_sns_governance::pb::v1::get_proposal_response::Result::Error(err) => {
            Err(CanisterError::Unknown(format!(
                "Governance error on proposal data: {}",
                err.error_message
            )))
        }
        ic_sns_governance::pb::v1::get_proposal_response::Result::Proposal(data) => {
            if data.reward_event_end_timestamp_seconds.is_some() {
                // proposal is not accepting votes anymore.
                // remove this proposal from the watchlist
                WATCHING_PROPOSALS
                    .with(|proposals| proposals.borrow_mut().retain(|proposal| proposal.id != id));
                participation_status = ParticipationStatus::TooLateToParticipate;
                // add this proposal and the final decision of the canister to the history
                PROPOSAL_HISTORY.with(|proposals| {
                    proposals.borrow_mut().push(ProxyProposalQuery {
                        id,
                        action,
                        creation_timestamp,
                        participation_status: participation_status.clone(),
                        timer_scheduled_for: None,
                    });
                });
                return Ok(participation_status);
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
                    vote(id, 0).await?;
                }
            } else {
                vote(id, 0).await?;
            }

            // remove this proposal from the watchlist
            WATCHING_PROPOSALS
                .with(|proposals| proposals.borrow_mut().retain(|proposal| proposal.id != id));

            // add this proposal and the final decision of the canister to the history
            PROPOSAL_HISTORY.with(|proposals| {
                proposals.borrow_mut().push(ProxyProposalQuery {
                    id,
                    action,
                    creation_timestamp,
                    participation_status: participation_status.clone(),
                    timer_scheduled_for: None,
                });
            });

            Ok(participation_status)
        }
    }
}
