use crate::*;
use near_sdk::{env, Promise, ext_contract, Gas, PromiseOrValue, assert_one_yocto, PromiseResult};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use::near_sdk::serde::{self, Serialize, Deserialize};
use near_sdk::AccountId;
use std::collections::HashMap;
use near_sdk::json_types::Base64VecU8;


pub type SalePriceInYoctoNear = U128;

const GAS_FOR_NFT_TRANSFER: Gas = Gas(30_000_000_000_000);

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Default, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetadata {

}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    //token ID
    pub token_id: TokenId,
    //owner of the token
    pub owner_id: AccountId,
    //token metadata
    pub metadata: TokenMetadata,
    //list of approved account IDs that have access to transfer the token. This maps an account ID to an approval ID
    pub approved_account_ids: HashMap<AccountId, u64>,
    //keep track of the royalty percentages for the token in a hash map
    pub royalty: Option<HashMap<AccountId, u32>>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SaleArgs {
    pub owner: AccountId,
	pub hash: String,
}

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_send_offer(&mut self,
	hash: String,
	sender_id: AccountId,
	sender_near: U128,
	sender_nfts: Vec<TokenData>,
	receiver_id: AccountId,
	receiver_nfts: Vec<TokenData>,
	is_holder: bool,
	) -> bool;
}

#[ext_contract(ext_nft_contract)]
trait NFTContract {
    fn nft_transfer(&mut self, receiver_id: AccountId, token_id: String);
    fn nft_token(&self, token_id: String) -> Option<Token>;
	fn nft_tokens_for_owner(&self, account_id: AccountId, from_index: String, limit: u16) -> Vec<Token>;
}

#[near_bindgen]
impl Contract {

	pub fn get_hashes_for_owner(&self, owner_id: AccountId) -> Vec<String> {
		let hashes_for_owner_set = self.hashes_per_owner.get(&owner_id);

		if let Some(hashes_for_owner_set) = hashes_for_owner_set {
			return hashes_for_owner_set;
		} else {
			return vec![];
		};
	}

	pub fn get_tokens_for_owner(&self, owner_id: AccountId) -> Vec<TokenData> {
		let tokens_per_owner_set = self.tokens_per_owner.get(&owner_id);

		if let Some(tokens_per_owner_set) = tokens_per_owner_set {
			return tokens_per_owner_set;
		} else {
			return vec![];
		};
	}

	pub fn get_transaction_data(&self, hash: Hash) -> Option<HashOffer> {
		let transaction_data = self.hash_map.get(&hash);

		if let Some(transaction_data) = transaction_data {
			return Some(transaction_data);
		} else {
			return None;
		};
	}

	#[payable]
	pub fn mass_transfer(
		&mut self,
		receiver_id: AccountId,
	) {
		let attached_deposit = env::attached_deposit();
		Promise::new(receiver_id).transfer(attached_deposit);
	}

	#[private]
	pub fn callback_send_offer(&mut self, hash: String,
		sender_id: AccountId,
		sender_near: U128,
		sender_nfts: Vec<TokenData>,
		receiver_id: AccountId,
		receiver_nfts: Vec<TokenData>,
		is_holder: bool) -> bool {
		
		match env::promise_result(0) {
			PromiseResult::NotReady => unreachable!(),
			PromiseResult::Successful(val) => {

				if let Ok(result) = near_sdk::serde_json::from_slice::<Vec<Token>>(&val) {
					let is_monarch = if result.len() > 0 {
						true
					} else {
						false
					};
					
					assert_eq!(
						is_holder,
						is_monarch,
						"Invalid holder status."
					);

					let account_id = &receiver_id;

					let transaction_data = HashOffer {
						sender_id: sender_id.clone(),
						sender_near: u128::from(sender_near),
						sender_nfts: sender_nfts,
						sent_nfts: Vec::new(),
						receiver_id: receiver_id.clone(),
						receiver_nfts: receiver_nfts,
						received_nfts: Vec::new(),
						timestamp: env::block_timestamp(),
						is_monarch: is_monarch,
					};

					self.hash_map.insert(&hash, &transaction_data);

					let mut hash_set = self.hashes_per_owner.get(&sender_id).unwrap_or_else(|| {
						Vec::new()
					});

					hash_set.push(hash.clone());

					let mut receiver_hash_set = self.hashes_per_owner.get(&account_id).unwrap_or_else(|| {
						Vec::new()
					});

					receiver_hash_set.push(hash.clone());

					self.hashes_per_owner.insert(&sender_id, &hash_set);
					self.hashes_per_owner.insert(&account_id, &receiver_hash_set);

					env::log_str(format!("Added offer: {}", &hash).as_str());
					true
				}
				else {
					env::panic_str("ERR_WRONG_VAL_RECEIVED");
					false
				}
			},
			PromiseResult::Failed => env::panic_str("ERR_CALLBACK_FAILED"),
		}


		
	}

	#[payable]
	pub fn send_offer( // deposit amount + args
		&mut self,
		hash: String,
		sender_id: AccountId,
		sender_near: U128,
		sender_nfts: Vec<TokenData>,
		receiver_id: AccountId,
		receiver_nfts: Vec<TokenData>,
		is_holder: bool,
	) -> Promise {
		let account = env::signer_account_id();
		let attached_deposit = env::attached_deposit();
		let required_cost = u128::from(self.required_cost);

		assert!(
			required_cost <= attached_deposit,
			"Must attach {} yoctoNEAR to cover costs",
			required_cost,
		);

		assert!(
			!self.hash_map.contains_key(&hash), // evals to false
			"Offer already exists",
		);

		if sender_near.0 < 10000000000000000000000000 || is_holder == true { // if below 10N collect 0.1N base fee
			if attached_deposit < sender_near.0 + required_cost {
				env::panic_str("Insufficient near attached");
			}
			
		}

		if is_holder == false {
			if sender_near.0 >= 10000000000000000000000000 { // if more than or equal to 10N collect 1% fee
				let tax = 100 as u128 * sender_near.clone().0 / 10_000u128;

				if attached_deposit < sender_near.0 + tax {
					env::panic_str("Insufficient near attached");
				}

			}
		}

		assert_eq!(
			account,
			sender_id,
			"Invalid sender"
		);

		assert_ne!(
			account,
			receiver_id,
			"Can't be receiver"
		);

		assert!(
			(sender_nfts.len() + receiver_nfts.len()) < 9,
			"Maximum NFTS per transaction is 8"
		);

		// let offer_amount = attached_deposit - required_cost;

		let promise = ext_nft_contract::ext(AccountId::try_from("mint.havendao.near".to_string()).unwrap()).nft_tokens_for_owner(account.clone(), format!("0"), 1);


		promise.then(
			Self::ext(env::current_account_id()).with_static_gas(GAS_FOR_NFT_TRANSFER)
			.callback_send_offer(hash, sender_id, sender_near, sender_nfts, receiver_id, receiver_nfts, is_holder)
		)



		// let promise1 = ext_nft_contract::ext(account.clone(),
		// format!("0"),
		// 1,
		// AccountId::try_from("mint.havendao.near".to_string()).unwrap(),
		// 0, 
		// GAS_FOR_NFT_TRANSFER).then(ext_self::callback_send_offer(
		// 	hash,
		// 	sender_id,
		// 	sender_near,
		// 	sender_nfts,
		// 	receiver_id,
		// 	receiver_nfts,
		// 	is_holder,
		// 	env::current_account_id().clone(),
		// 	0,
		// 	GAS_FOR_NFT_TRANSFER
		// ));

		// return promise;
		
	}

	// #[payable]
	// pub fn withdraw(
	// 	&mut self,
	// 	target_id: AccountId,
	// 	amount: U128,
	// ) {
	// 	self.assert_owner();

	// 	assert!(
	// 		self.user_deposits.contains_key(&target_id),
	// 		"No amount found to withdraw"
	// 	);

	// 	let stored_amount = self.user_deposits.get(&target_id).unwrap();

	// 	self.user_deposits.remove(&target_id);

	// 	Promise::new(target_id).transfer(u128::from(stored_amount));
	// 	env::log_str(format!("Sent {:?} yoctoNEAR", stored_amount).as_str())
	// }

	#[payable]
	pub fn nft_on_transfer(
		&mut self,
		sender_id: AccountId,
		previous_owner_id: AccountId,
        token_id: TokenId,
        msg: Hash,
	) -> PromiseOrValue<bool> {
		// get the contract ID which is the predecessor
        let nft_contract_id = env::predecessor_account_id();
        //get the signer which is the person who initiated the transaction
        let signer_id = env::signer_account_id();

        //make sure that the signer isn't the predecessor. This is so that we're sure
        //this was called via a cross-contract call
        assert_ne!(
            nft_contract_id,
            signer_id,
            "nft_on_transfer should only be called via cross-contract call"
        );

		//make sure the owner ID is the signer. 
        assert_eq!(
            previous_owner_id,
            signer_id,
            "owner_id should be signer_id"
        );
		
		let hash_set = self.hashes_per_owner.get(&signer_id);

		assert!(
			hash_set.is_some(),
			"{} is not initialized",
			signer_id
		);

		let hash_vec = hash_set.unwrap();

		assert!(
			hash_vec.contains(&msg),
			"Hash not found!"
		);

		let mut hash_transaction = self.hash_map.get(&msg).unwrap();

		assert!(
			signer_id == hash_transaction.sender_id || signer_id == hash_transaction.receiver_id,
			"Signer is not sender or receiver",
		);

		if &signer_id == &hash_transaction.sender_id {
			let expected_nfts = hash_transaction.sender_nfts.clone();

			let found_token: Vec<TokenData> = expected_nfts.clone()
				.into_iter()
				.filter(|value| value.contract_id == nft_contract_id && value.token_id == token_id)
				.collect();

			assert!(
				found_token.len() > 0,
				"Wrong nft sent"
			);

			let token_data = TokenData { contract_id: nft_contract_id.clone(), token_id: token_id.clone() };

			hash_transaction.sent_nfts.push(token_data.clone());

			self.hash_map.insert(&msg, &hash_transaction);

			let mut tokens_per_owner_vec = self.tokens_per_owner.get(&signer_id).unwrap_or_else(|| {
				Vec::new()
			});

			tokens_per_owner_vec.push(token_data);

			self.tokens_per_owner.insert(&signer_id, &tokens_per_owner_vec);

		}
		else if &signer_id == &hash_transaction.receiver_id {
			let expected_nfts = hash_transaction.receiver_nfts.clone();

			let found_token: Vec<TokenData> = expected_nfts.clone()
				.into_iter()
				.filter(|value| value.contract_id == nft_contract_id && value.token_id == token_id)
				.collect();

			assert!(
				found_token.len() > 0,
				"Wrong nft sent"
			);

			let token_data = TokenData { contract_id: nft_contract_id.clone(), token_id: token_id.clone() };

			hash_transaction.received_nfts.push(token_data.clone());

			self.hash_map.insert(&msg, &hash_transaction);

			let mut tokens_per_owner_vec = self.tokens_per_owner.get(&hash_transaction.receiver_id).unwrap_or_else(|| {
				Vec::new()
			});

			tokens_per_owner_vec.push(token_data);

			self.tokens_per_owner.insert(&hash_transaction.receiver_id, &tokens_per_owner_vec);

		}
			
		let tx_stored = self.hash_map.get(&msg).unwrap();
		let sent_nft_count = tx_stored.sent_nfts.len() as isize - tx_stored.sender_nfts.len() as isize ;
		let receive_nft_count = tx_stored.received_nfts.len() as isize - tx_stored.receiver_nfts.len() as isize ;

		if sent_nft_count != 0 {
			env::log_str(format!("sender hasnt sent all nfts").as_str());
			return PromiseOrValue::Value(false)
		}

		if receive_nft_count != 0 {
			env::log_str(format!("receiver hasnt sent all nfts").as_str());
			return PromiseOrValue::Value(false)
		}

		// all nfts have been sent
		let sender_array = tx_stored.sent_nfts;
		let receiver_array = tx_stored.received_nfts;

		let mut temp_tokens_arr = self.tokens_per_owner.get(&tx_stored.sender_id);

		if temp_tokens_arr.is_some() {
			let mut tokens_arr = self.tokens_per_owner.get(&tx_stored.sender_id).unwrap();
			
			for nfts_data in sender_array.iter() {
				ext_nft_contract::ext(nfts_data.contract_id.clone()).nft_transfer(tx_stored.receiver_id.clone(), nfts_data.token_id.clone());
				let tokens_index = tokens_arr.iter().position(|x| *x.token_id == nfts_data.token_id.clone()).unwrap();
				tokens_arr.remove(tokens_index);
			}

			self.tokens_per_owner.remove(&tx_stored.sender_id);
			self.tokens_per_owner.insert(&tx_stored.sender_id, &tokens_arr);
		}

		let mut temp_tokens_arr2 = self.tokens_per_owner.get(&tx_stored.receiver_id);

		if temp_tokens_arr2.is_some() {
			let mut tokens_arr2 = self.tokens_per_owner.get(&tx_stored.receiver_id).unwrap();

			for nfts_data in receiver_array.iter() {
				ext_nft_contract::ext(nfts_data.contract_id.clone()).nft_transfer(tx_stored.sender_id.clone(), nfts_data.token_id.clone());
				let tokens_index2 = tokens_arr2.iter().position(|x| *x.token_id == nfts_data.token_id.clone()).unwrap();
				tokens_arr2.remove(tokens_index2);
			}

			self.tokens_per_owner.remove(&tx_stored.receiver_id);
			self.tokens_per_owner.insert(&tx_stored.receiver_id, &tokens_arr2);
		}

		let mut hash_arr = self.hashes_per_owner.get(&tx_stored.sender_id).unwrap();
		let index = hash_arr.iter().position(|x| *x == msg.clone()).unwrap();
		hash_arr.remove(index);

		self.hashes_per_owner.remove(&tx_stored.sender_id);
		self.hashes_per_owner.insert(&tx_stored.sender_id, &hash_arr);


		let mut hash_arr2 = self.hashes_per_owner.get(&tx_stored.receiver_id).unwrap();
		let index2 = hash_arr2.iter().position(|x| *x == msg.clone()).unwrap();
		hash_arr2.remove(index2);
		
		self.hashes_per_owner.remove(&tx_stored.receiver_id);
		self.hashes_per_owner.insert(&tx_stored.receiver_id, &hash_arr2);

		let is_monarch = tx_stored.is_monarch;

		if tx_stored.sender_near > 0 {
			// transfer near to the muhfucker
			let tax = 100 as u128 * tx_stored.sender_near / 10_000u128;

			if tx_stored.sender_near >= 10000000000000000000000000 {
				if is_monarch == false {
					Promise::new(self.fee_wallet.clone()).transfer(tax);
				}
				else {
					Promise::new(self.fee_wallet.clone()).transfer(self.required_cost.clone().0);
				}
			}
			else {
				Promise::new(self.fee_wallet.clone()).transfer(self.required_cost.clone().0);
			}

			Promise::new(tx_stored.receiver_id.clone()).transfer(tx_stored.sender_near);
		}
		else {
			Promise::new(self.fee_wallet.clone()).transfer(self.required_cost.clone().0);

		}
		
		self.hash_map.remove(&msg);
		return PromiseOrValue::Value(false)
	}

	#[payable]
	pub fn cancel_offer(
		&mut self,
		hash: Hash,
	) {
		assert_one_yocto();

		let mut hash_transaction = self.hash_map.get(&hash).unwrap();

		let is_monarch = hash_transaction.is_monarch;


		let signer_id = env::signer_account_id();

		if signer_id == env::current_account_id() {

			let signer_nfts = hash_transaction.sent_nfts;
			let receiver_nfts = hash_transaction.received_nfts;


			if self.tokens_per_owner.get(&hash_transaction.sender_id).is_some() {
				let mut tokens_arr = self.tokens_per_owner.get(&hash_transaction.sender_id).unwrap();

				for nfts_data in signer_nfts.iter() {
					let tokens_index = tokens_arr.iter().position(|x| *x.token_id == nfts_data.token_id.clone()).unwrap();
					tokens_arr.remove(tokens_index);
				}

				// remove token from storage
				self.tokens_per_owner.remove(&hash_transaction.sender_id);
				self.tokens_per_owner.insert(&hash_transaction.sender_id, &tokens_arr);
			}
			
			if self.tokens_per_owner.get(&hash_transaction.receiver_id).is_some() {
				let mut tokens_arr2 = self.tokens_per_owner.get(&hash_transaction.receiver_id).unwrap();

				for nfts_data in receiver_nfts {
					let tokens_index = tokens_arr2.iter().position(|x| *x.token_id == nfts_data.token_id.clone()).unwrap();
					tokens_arr2.remove(tokens_index);
				}

				// remove token from storage
				self.tokens_per_owner.remove(&hash_transaction.receiver_id);
				self.tokens_per_owner.insert(&hash_transaction.receiver_id, &tokens_arr2);
			}

			let mut hash_arr = self.hashes_per_owner.get(&hash_transaction.sender_id).unwrap();
			let index = hash_arr.iter().position(|x| *x == hash.clone()).unwrap();
			hash_arr.remove(index);

			// remove hash from storage
			self.hashes_per_owner.remove(&hash_transaction.sender_id);
			self.hashes_per_owner.insert(&hash_transaction.sender_id, &hash_arr);

			
			let mut hash_arr = self.hashes_per_owner.get(&hash_transaction.receiver_id).unwrap();
			let index2 = hash_arr.iter().position(|x| *x == hash.clone()).unwrap();
			hash_arr.remove(index2);

			// remove hash from storage
			self.hashes_per_owner.remove(&hash_transaction.receiver_id);
			self.hashes_per_owner.insert(&hash_transaction.receiver_id, &hash_arr);

			self.hash_map.remove(&hash);

			env::log_str(format!("Cancelled transaction: {}", &hash).as_str());
			return
		}



		let hash_set = self.hashes_per_owner.get(&signer_id);

		assert!(
			hash_set.is_some(),
			"{} is not initialized",
			signer_id
		);

		let hash_vec = hash_set.unwrap();

		assert!(
			hash_vec.contains(&hash),
			"Hash not found!"
		);

		assert!(
			&signer_id == &hash_transaction.sender_id || &signer_id == &hash_transaction.receiver_id,
			"Signer is not sender or receiver",
		);

		let signer_nfts = hash_transaction.sent_nfts;
		let receiver_nfts = hash_transaction.received_nfts;

		if self.tokens_per_owner.get(&hash_transaction.sender_id).is_some() {
			let mut tokens_arr = self.tokens_per_owner.get(&hash_transaction.sender_id).unwrap();

			for nfts_data in signer_nfts.iter() {
				ext_nft_contract::ext(nfts_data.contract_id.clone()).nft_transfer(hash_transaction.sender_id.clone(), nfts_data.token_id.clone());
				// ext_nft_contract::nft_transfer(hash_transaction.sender_id.clone(), nfts_data.token_id.clone(), nfts_data.contract_id.clone(), 1, GAS_FOR_NFT_TRANSFER);
				let tokens_index = tokens_arr.iter().position(|x| *x.token_id == nfts_data.token_id.clone()).unwrap();
				tokens_arr.remove(tokens_index);
			}

			// remove token from storage
			self.tokens_per_owner.remove(&hash_transaction.sender_id);
			self.tokens_per_owner.insert(&hash_transaction.sender_id, &tokens_arr);
		}
		
		
		if self.tokens_per_owner.get(&hash_transaction.receiver_id).is_some() {
			let mut tokens_arr2 = self.tokens_per_owner.get(&hash_transaction.receiver_id).unwrap();

			for nfts_data in receiver_nfts.iter() {
				ext_nft_contract::ext(nfts_data.contract_id.clone()).nft_transfer(hash_transaction.receiver_id.clone(), nfts_data.token_id.clone());
				// ext_nft_contract::nft_transfer(hash_transaction.receiver_id.clone(), nfts_data.token_id.clone(), nfts_data.contract_id.clone(), 1, GAS_FOR_NFT_TRANSFER);
				let tokens_index = tokens_arr2.iter().position(|x| *x.token_id == nfts_data.token_id.clone()).unwrap();
				tokens_arr2.remove(tokens_index);
			}

			// remove token from storage
			self.tokens_per_owner.remove(&hash_transaction.receiver_id);
			self.tokens_per_owner.insert(&hash_transaction.receiver_id, &tokens_arr2);
		}

		let mut hash_arr = self.hashes_per_owner.get(&hash_transaction.sender_id).unwrap();
		let index = hash_arr.iter().position(|x| *x == hash.clone()).unwrap();
		hash_arr.remove(index);

		// remove hash from storage
		self.hashes_per_owner.remove(&hash_transaction.sender_id);
		self.hashes_per_owner.insert(&hash_transaction.sender_id, &hash_arr);

		
		let mut hash_arr = self.hashes_per_owner.get(&hash_transaction.receiver_id).unwrap();
		let index2 = hash_arr.iter().position(|x| *x == hash.clone()).unwrap();
		hash_arr.remove(index2);

		// remove hash from storage
		self.hashes_per_owner.remove(&hash_transaction.receiver_id);
		self.hashes_per_owner.insert(&hash_transaction.receiver_id, &hash_arr);

		if hash_transaction.sender_near > 0 {

			if hash_transaction.sender_near >= 10000000000000000000000000 { // if more than or equal to 10N collect 1% fee
				let tax = 100 as u128 * hash_transaction.sender_near.clone() / 10_000u128;

				if tax > self.required_cost.clone().0 {
					let sent_amount = tax - self.required_cost.clone().0;

					if is_monarch {
						Promise::new(hash_transaction.sender_id.clone()).transfer(self.required_cost.clone().0);
					}
					else {
						Promise::new(hash_transaction.sender_id.clone()).transfer(sent_amount);
					}
				}

			}
			Promise::new(self.fee_wallet.clone()).transfer(self.required_cost.clone().0);

			Promise::new(hash_transaction.sender_id.clone()).transfer(hash_transaction.sender_near);
		}
		else {
			Promise::new(self.fee_wallet.clone()).transfer(self.required_cost.clone().0);
		}

		self.hash_map.remove(&hash);

		env::log_str(format!("Cancelled transaction: {}", &hash).as_str());
	}
	// #[payable]
	// pub fn deposit_deduct( // deduct per transaction
	// 	&mut self,
	// 	target_id: AccountId,
	// 	amount: Option<U128>,
	// ) {
	// 	self.assert_owner();

	// 	let deposit_amount = u128::from(self.user_deposits.get(&target_id).unwrap());

	// 	let required_cost = if !amount.is_none() { u128::from(amount.unwrap()) } else { u128::from(self.required_cost) };

	// 	assert!(
	// 		required_cost < deposit_amount,
	// 		"Insufficient amount in deposit to continue",
	// 	);

	// 	let new_amount = u128::from(self.user_deposits.remove(&target_id).unwrap());

	// 	if new_amount >= required_cost {
	// 		self.user_deposits.insert(&target_id, &U128(new_amount - required_cost));
	// 	}

	// 	env::log_str(format!("Successfully deducted from {}", &target_id).as_str())
	// }
}