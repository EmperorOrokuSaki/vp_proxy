mod canister;

use crate::canister::VpProxy;

fn main() {
    let canister_e_idl = VpProxy::idl();
    let idl = candid::pretty::candid::compile(&canister_e_idl.env.env, &Some(canister_e_idl.actor));

    println!("{}", idl);
}
