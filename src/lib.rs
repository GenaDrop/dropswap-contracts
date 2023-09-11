use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::{near_bindgen, AccountId, require, env};
use::near_sdk::serde::{self, Serialize, Deserialize};


pub use crate::account::*;

mod account; 

pub type Hash = String;
pub type TokenId = String;


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Contract {
	pub hash_map: LookupMap<Hash, HashOffer>,
	pub hashes_per_owner: LookupMap<AccountId, Vec<Hash>>,
	pub tokens_per_owner: LookupMap<AccountId, Vec<TokenData>>,
	pub owner_id: String,
	pub fee_wallet: AccountId,
	pub required_cost: U128,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct HashOffer {
	pub sender_id: AccountId,
	pub sender_near: u128,
	pub sender_nfts: Vec<TokenData>,
	pub sent_nfts: Vec<TokenData>,
	pub receiver_id: AccountId,
	pub receiver_nfts: Vec<TokenData>,
	pub received_nfts: Vec<TokenData>,
	pub timestamp: u64,
	pub is_monarch: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenData {
	pub contract_id: AccountId,
	pub token_id: TokenId,
}

impl Default for Contract {
	fn default() -> Self {
		Self {
			hash_map: LookupMap::new(b"hash_map".to_vec()),
			hashes_per_owner: LookupMap::new(b"hashes_per_owner".to_vec()),
			tokens_per_owner: LookupMap::new(b"tokens_per_owner".to_vec()),
			owner_id: "v1.havenswap.near".to_string(), // change me
			fee_wallet: AccountId::new_unchecked("fee.havenswap.near".to_string()), // change me
			required_cost: U128(100000000000000000000000),
		}
	}
}

#[near_bindgen]
impl Contract {
    // ADD CONTRACT METHODS HERE
	fn assert_owner(&self) {
        require!(self.signer_is_owner(), "Method is private to owner")
    }

    fn signer_is_owner(&self) -> bool {
        self.is_owner(&env::signer_account_id())
    }

    fn is_owner(&self, account: &AccountId) -> bool {
        account.to_string() == self.owner_id
    }
}

/*
 * the rest of this file sets up unit tests
 * to run these, the command will be:
 * cargo test --package rust-template -- --nocapture
 * Note: 'rust-template' comes from Cargo.toml's 'name' key
 */

// use the attribute below for unit tests
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_logs, VMContextBuilder};
    use near_sdk::{testing_env, AccountId};

    // part of writing unit tests is setting up a mock context
    // provide a `predecessor` here, it'll modify the default context
    fn get_context(predecessor: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder.predecessor_account_id(predecessor);
        builder
    }

    // TESTS HERE
}