use ic_canister::{generate_idl, init, query, update, Canister, Idl, PreUpdate};
use ic_exports::{candid::Principal, ic_cdk::caller, ic_kit::ic::time};

#[derive(Canister)]
pub struct VpProxy {
    #[id]
    id: Principal,
}

impl PreUpdate for VpProxy {}

impl VpProxy {
    // INITIALIZATION
    
    pub fn idl() -> Idl {
        generate_idl!()
    }
}
