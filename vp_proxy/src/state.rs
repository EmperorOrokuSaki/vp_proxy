use std::cell::RefCell;

use ic_exports::ic_kit::Principal;

use crate::types::CouncilMember;

thread_local! {
    pub static COUNCIL_MEMBERS: RefCell<Vec<CouncilMember>> = RefCell::new(Vec::new());
    pub static GOVERNANCE_CANISTER_ID: RefCell<Principal> = RefCell::new(Principal::anonymous()); // should be set via set_governance(id: Principal)
}