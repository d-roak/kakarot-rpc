use std::str::FromStr;

use crate::models::felt::Felt252Wrapper;
use alloy_rlp::Encodable;
use cainome::rs::abigen_legacy;
use dotenv::dotenv;
use lazy_static::lazy_static;
use reth_primitives::{Address, Transaction, TransactionSigned};
use starknet::{
    core::{types::BroadcastedInvokeTransactionV1, utils::get_contract_address},
    macros::selector,
};
use starknet_crypto::FieldElement;

use crate::{
    eth_provider::{provider::EthProviderResult, utils::split_u256},
    into_via_wrapper,
};

// Contract ABIs

pub mod proxy {
    use super::*;
    abigen_legacy!(Proxy, "./.kakarot/artifacts/proxy.json");
}

pub mod contract_account {
    use super::*;
    abigen_legacy!(ContractAccount, "./.kakarot/artifacts/contract_account.json");
}

#[allow(clippy::too_many_arguments)]
pub mod core {
    use super::*;
    abigen_legacy!(KakarotCore, "./.kakarot/artifacts/kakarot.json");
}

fn env_var_to_field_element(var_name: &str) -> FieldElement {
    dotenv().ok();
    let env_var = std::env::var(var_name).unwrap_or_else(|_| panic!("Missing environment variable {var_name}"));

    FieldElement::from_str(&env_var).unwrap_or_else(|_| panic!("Invalid hex string for {var_name}"))
}

lazy_static! {
    // Contract addresses
    pub static ref KAKAROT_ADDRESS: FieldElement = env_var_to_field_element("KAKAROT_ADDRESS");

    // Contract class hashes
    pub static ref PROXY_ACCOUNT_CLASS_HASH: FieldElement = env_var_to_field_element("PROXY_ACCOUNT_CLASS_HASH");
    pub static ref EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH: FieldElement =
        env_var_to_field_element("EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH");
    pub static ref CONTRACT_ACCOUNT_CLASS_HASH: FieldElement = env_var_to_field_element("CONTRACT_ACCOUNT_CLASS_HASH");

    // Contract selectors
    pub static ref ETH_SEND_TRANSACTION: FieldElement = selector!("eth_send_transaction");
}

// Kakarot utils
/// Compute the starknet address given a eth address
pub fn starknet_address(address: Address) -> FieldElement {
    get_contract_address(into_via_wrapper!(address), *PROXY_ACCOUNT_CLASS_HASH, &[], *KAKAROT_ADDRESS)
}

/// Convert a Ethereum transaction into a Starknet transaction
pub fn to_starknet_transaction(
    transaction: &TransactionSigned,
    chain_id: u64,
    signer: Address,
    max_fee: u64,
) -> EthProviderResult<BroadcastedInvokeTransactionV1> {
    let starknet_address = starknet_address(signer);

    let nonce = FieldElement::from(transaction.nonce());

    // Step: Signature
    // Extract the signature from the Ethereum Transaction
    // and place it in the Starknet signature InvokeTransaction vector
    let mut signature: Vec<FieldElement> = {
        let r = split_u256(transaction.signature().r);
        let s = split_u256(transaction.signature().s);
        let mut signature = Vec::with_capacity(5);
        signature.extend_from_slice(&r);
        signature.extend_from_slice(&s);
        signature
    };
    // Push the last element of the signature
    // In case of a Legacy Transaction, it is v := {0, 1} + chain_id * 2 + 35
    // Else, it is odd_y_parity
    if let Transaction::Legacy(_) = transaction.transaction {
        signature.push(transaction.signature().v(Some(chain_id)).into());
    } else {
        signature.push((transaction.signature().odd_y_parity as u64).into());
    }

    // Step: Calldata
    // RLP encode the transaction without the signature
    // Example: For Legacy Transactions: rlp([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0])
    let mut signed_data = Vec::with_capacity(transaction.transaction.length());
    transaction.transaction.encode_without_signature(&mut signed_data);

    // Prepare the calldata for the Starknet invoke transaction
    let capacity = 6 + signed_data.len();
    let mut execute_calldata = Vec::with_capacity(capacity);
    execute_calldata.append(&mut vec![
        FieldElement::ONE,                     // call array length
        *KAKAROT_ADDRESS,                      // contract address
        *ETH_SEND_TRANSACTION,                 // selector
        FieldElement::ZERO,                    // data offset
        FieldElement::from(signed_data.len()), // data length
        FieldElement::from(signed_data.len()), // calldata length
    ]);
    execute_calldata.append(&mut signed_data.into_iter().map(FieldElement::from).collect());

    Ok(BroadcastedInvokeTransactionV1 {
        max_fee: max_fee.into(),
        signature,
        nonce,
        sender_address: starknet_address,
        calldata: execute_calldata,
        is_query: false,
    })
}
