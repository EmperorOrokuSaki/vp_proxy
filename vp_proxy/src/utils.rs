use ic_exports::{
    ic_cdk::{api::is_controller, call, print},
    ic_kit::{CallResult, Principal},
};
use ic_sns_governance::pb::v1::{
    manage_neuron::{self, RegisterVote},
    ManageNeuron, ManageNeuronResponse, ProposalId,
};

use crate::{
    state::{get_governance_canister_id, get_neuron, GOVERNANCE_CANISTER_ID},
    types::CanisterError,
};

pub fn only_controller(caller: Principal) -> Result<(), CanisterError> {
    if !is_controller(&caller) {
        // only the controller (the DAO) should be able to call this function
        return Err(CanisterError::Unauthorized);
    }
    Ok(())
}

pub fn not_anonymous(id: &Principal) -> Result<(), CanisterError> {
    if id == &Principal::anonymous() {
        return Err(CanisterError::ConfigurationError);
    }
    Ok(())
}

pub fn handle_intercanister_call<T>(
    canister_response: CallResult<(T,)>,
) -> Result<T, CanisterError> {
    match canister_response {
        Ok((response,)) => Ok(response),
        Err((_code, message)) => Err(CanisterError::Unknown(message)),
    }
}

pub async fn vote(proposal_id: ProposalId, vote: i32) -> Result<(), CanisterError> {
    let neuron = get_neuron()?;
    let governance_canister_id = get_governance_canister_id()?;

    let register_vote_args = ManageNeuron {
        subaccount: neuron.id,
        command: Some(manage_neuron::Command::RegisterVote(RegisterVote {
            proposal: Some(proposal_id),
            vote,
        })),
    };

    let register_vote_response = call(
        governance_canister_id,
        "manage_neuron",
        (register_vote_args,),
    )
    .await;

    let manage_neuron_response =
        handle_intercanister_call::<ManageNeuronResponse>(register_vote_response)?;

    if let Some(command) = manage_neuron_response.command {
        return Ok(());
    }
    Err(CanisterError::Unknown(
        "Could not handle the manage neuron response".to_string(),
    ))
}
