use std::cell::{Cell, RefCell};

use ic_exports::{ic_cdk_timers::TimerId, ic_kit::Principal};
use ic_sns_governance::pb::v1::{NeuronId, ProposalId};

use crate::{
    types::{CanisterError, CouncilMember, ProxyProposal, ProxyProposalQuery},
    utils::not_anonymous,
};

thread_local! {
    /// Watching status for new proposals
    pub static WATCH_LOCK: Cell<bool> = Cell::new(false);
    /// Fetcher recurring timer's ID
    pub static FETCHER_TIMER_ID: RefCell<Option<TimerId>> = RefCell::new(None);
    /// The DAO's governance canister's principal ID.
    pub static GOVERNANCE_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_governance_id(id: Principal)
    /// The token ledger canister's principal ID.
    pub static LEDGER_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_ledger_id(id: Principal)
    /// Max number of retries the proxy canister will attempt, if anything fails.
    pub static MAX_RETRIES: Cell<u8> = Cell::new(3);
    /// Vector of all current council members
    pub static COUNCIL_MEMBERS: RefCell<Vec<CouncilMember>> = RefCell::new(Vec::new());
    /// Proposals that are currently being watched (a one-off timer will be triggered one hour before the voting deadline)
    pub static WATCHING_PROPOSALS: RefCell<Vec<ProxyProposal>> = RefCell::new(Vec::new());
    /// Proposals that had been watched.
    pub static PROPOSAL_HISTORY: RefCell<Vec<ProxyProposalQuery>> = RefCell::new(Vec::new());
    /// Actions that will be ignored (the proxy canister won't vote on proposals that have an action from this list)
    pub static EXCLUDED_ACTION_IDS: RefCell<Vec<u64>> = RefCell::new(Vec::new());
    /// The last proposal that was handled in this canister.
    pub static LAST_PROPOSAL: RefCell<Option<ProxyProposalQuery>> = RefCell::new(None);
    /// The proxy canister's neuron ID.
    pub static NEURON_ID: RefCell<Option<NeuronId>> = RefCell::new(None);
}

pub fn is_proposal_locked(id: ProposalId) -> Option<bool> {
    let mut lock = None;

    WATCHING_PROPOSALS.with(|proposals| {
        let _ = proposals.borrow().iter().map(|proposal_data| {
            if proposal_data.id == id {
                lock = Some(proposal_data.lock)
            }
        });
    });

    lock
}

pub fn get_fetcher_timer_id() -> Option<TimerId> {
    FETCHER_TIMER_ID.with(|id| id.borrow().clone())
}

pub fn get_watch_lock() -> bool {
    WATCH_LOCK.with(|lock| lock.get())
}

pub fn get_exclusion_list() -> Vec<u64> {
    EXCLUDED_ACTION_IDS.with(|actions| actions.borrow().clone())
}

pub fn get_neuron() -> Result<NeuronId, CanisterError> {
    let neuron_id = NEURON_ID.with(|id| id.borrow().clone());
    if neuron_id.is_some() {
        return Ok(neuron_id.unwrap());
    }
    Err(CanisterError::Unknown("Undefined neuron id".to_string()))
}

pub fn get_proposal_watchlist() -> Vec<ProxyProposalQuery> {
    WATCHING_PROPOSALS.with(|proposals| {
        proposals
            .borrow()
            .iter()
            .map(|proposal| proposal.clone().into())
            .collect()
    })
}

pub fn get_proposal_history() -> Vec<ProxyProposalQuery> {
    PROPOSAL_HISTORY.with(|proposals| {
        proposals
            .borrow()
            .iter()
            .map(|proposal| proposal.clone().into())
            .collect()
    })
}

pub fn get_council_members() -> Vec<CouncilMember> {
    COUNCIL_MEMBERS.with(|members| members.borrow().clone())
}

pub fn get_max_retries() -> u8 {
    MAX_RETRIES.with(|count| count.get())
}

pub fn get_governance_canister_id() -> Result<Principal, CanisterError> {
    let governance_canister_id = GOVERNANCE_CANISTER_ID.with(|id| id.borrow().clone());
    not_anonymous(&governance_canister_id)?;
    Ok(governance_canister_id)
}

pub fn get_ledger_canister_id() -> Result<Principal, CanisterError> {
    let ledger_canister_id = LEDGER_CANISTER_ID.with(|id| id.borrow().clone());
    not_anonymous(&ledger_canister_id)?;
    Ok(ledger_canister_id)
}

pub fn get_last_proposal_id() -> Result<ProxyProposalQuery, CanisterError> {
    let last_proposal_id = LAST_PROPOSAL.with(|id| id.borrow().clone());
    if last_proposal_id.is_some() {
        return Ok(last_proposal_id.unwrap());
    }
    Err(CanisterError::Unknown(
        "Undefined last proposal id.".to_string(),
    ))
}
