mod canister;
mod types;
mod state;
mod utils;

use crate::canister::VpProxy;

fn main() {
    let canister_e_idl = VpProxy::idl();
    let idl = ic_exports::candid::pretty::candid::compile(&canister_e_idl.env.env, &Some(canister_e_idl.actor));

    println!("{}", idl);
}
