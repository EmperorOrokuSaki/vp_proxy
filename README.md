# Voting Power Proxy Canister

## Overview

The Voting Power (VP) Proxy Canister enables council members of SNS DAOs to manage a specified amount of staked SNS tokens within a neuron. It can be configured to participate only in proposals that meet certain criteria, such as not having a specific action type and proposals with titles not beginning with "CONFIGURE COUNCIL NEURON".

## Specification

### Voting Criteria

- The canister votes on a proposal only if at least 50% of all council neurons have participated.
  - If less than 50% have participated, the proxy abstains from voting.
- If the participation condition is met:
  - The proxy votes in favor of the proposal if more than 50% of the participating council members have voted yes.
  - Otherwise, the proxy votes against the proposal.

### Listening to Proposals

Once activated via the `watch_proposals` method, the proxy starts a recurring timer that checks for new proposals every 24 hours.

#### Filtering Proposals

The proxy canister excludes the following proposals:

- Proposals with an action ID listed in the exclusion list.
- Proposals with a title starting with "CONFIGURE COUNCIL NEURON".

#### Handling Proposals

When a new proposal is added to the watchlist, a one-time timer is set to trigger one hour before the proposal's voting deadline. At that time, the proxy evaluates the participation of council neurons and decides the verdict if voting is still open.

## Deployment

The canister can be deployed by anyone, not just the DAO. Follow these steps to deploy:

1. Deploy the canister on the IC mainnet: 
    ```sh
    dfx deploy --ic
    ```
2. Configure the SNS governance canister: 
    ```sh
    dfx canister call --ic vp_proxy set_governance_id '(principal "PID")'
    ```
3. Configure the SNS ledger canister: 
    ```sh
    dfx canister call --ic vp_proxy set_ledger_id '(principal "PID")'
    ```
4. Add a council member: 
    ```sh
    dfx canister call --ic vp_proxy add_council_member '("NAME", "NEURON-ID")'
    ```
5. Create a neuron after sending SNS tokens to the canister: 
    ```sh
    dfx canister call --ic vp_proxy create_neuron '(TOKEN_AMOUNT, NONCE)'
    ```
6. Add action types to the exclusion list: 
    ```sh
    dfx canister call --ic vp_proxy disallow_action_type '(ACTION_TYPE_ID)'
    ```
7. Start listening to incoming proposals from a given proposal ID: 
    ```sh
    dfx canister call --ic vp_proxy watch_proposals '(record { id = PROPOSAL_ID }, FROM_PROPOSAL_ACTION, FROM_PROPOSAL_CREATION_TIMESTAMP)'
    ```

### Additional Configuration

- Emergency reset of council members: 
    ```sh
    dfx canister call --ic vp_proxy emergency_reset
    ```
- Allow a previously excluded action type: 
    ```sh
    dfx canister call --ic vp_proxy allow_action_type '(ACTION_TYPE_ID)'
    ```
- Remove a previously appointed council member: 
    ```sh
    dfx canister call --ic vp_proxy remove_council_member '(NEURON_ID)'
    ```

### Queries

The canister exposes the following query methods:

- List all council members: 
    ```sh
    dfx canister call --ic vp_proxy get_council
    ```
- List all proposals on the watchlist: 
    ```sh
    dfx canister call --ic vp_proxy get_proposal_watchlist
    ```
- List all proposals that were on the watchlist: 
    ```sh
    dfx canister call --ic vp_proxy get_proposal_history
    ```
- Get the status of a specific proposal by its ID: 
    ```sh
    dfx canister call --ic vp_proxy get_proposal_status '(record {id = PROPOSAL_ID})'
    ```
- List all excluded action types: 
    ```sh
    dfx canister call --ic vp_proxy get_exclusion_list
    ```

## Acknowledgments

This canister was developed for the ICP CC DAO. However, any other SNS DAO or individual who wishes to use it for personal reasons is welcome to do so.