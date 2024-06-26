mod core;
mod storage;

use near_contract_standards::fungible_token::{
    events::{FtBurn, FtMint},
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider},
    FungibleToken, FungibleTokenResolver,
};
use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, require, AccountId, BorshStorageKey, PanicOnDefault, Promise,
};

#[derive(BorshStorageKey, BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
pub enum StorageKey {
    Token,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[borsh(crate = "near_sdk::borsh")]
pub struct Contract {
    owner: AccountId,
    migrate_address: AccountId,
    token: FungibleToken,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner: AccountId, migrate_address: AccountId) -> Self {
        Self {
            owner,
            migrate_address,
            token: FungibleToken::new(StorageKey::Token),
        }
    }

    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        require!(
            env::block_timestamp() < 1_718_409_600_000_000_000,
            "Migration closed indefinitely"
        );
        require!(
            env::predecessor_account_id() == self.migrate_address,
            "Only Shitzu address on Aurora can call this function"
        );
        if !self.token.accounts.contains_key(&account_id) {
            self.token.internal_register_account(&account_id);
        }
        self.token.internal_deposit(&account_id, amount.into());
        FtMint {
            owner_id: &account_id,
            amount,
            memo: None,
        }
        .emit();
    }

    /// This function recovers tokens that have been accidentially sent to the contract address itself
    /// and will send those tokens back to the contract owner's address.
    pub fn recover(&mut self) {
        let self_id = env::current_account_id();
        let balance = self.token.internal_unwrap_balance_of(&self_id);
        require!(balance > 0, "Balance is zero");
        self.token.internal_withdraw(&self_id, balance);
        self.token.internal_deposit(&self.owner, balance);
    }

    /// Since within4d45 has sent his burner account balance to account '114155'
    /// instead sending 114155 tokens, he created this recovery function
    /// See tx: 2zmB5uumyaUf4hzCeDyaqH81Fpk9b2iRoAZq2Na3bP3C
    pub fn recover_within(&mut self) {
        let id: AccountId = "114155".parse().unwrap();
        let balance = self.token.internal_unwrap_balance_of(&id);
        require!(balance > 0, "Balance is zero");
        self.token.internal_withdraw(&id, balance);
        self.token.internal_deposit(&self.owner, balance);
    }

    pub fn migrate(&mut self) {
        // empty for now
    }

    pub fn upgrade(&self) -> Promise {
        require!(
            env::predecessor_account_id() == self.owner,
            "Only account owner can update the code"
        );

        let code = env::input().expect("Error: No input").to_vec();

        Promise::new(env::current_account_id())
            .deploy_contract(code)
            .then(Self::ext(env::current_account_id()).migrate())
            .as_return()
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, burned_amount) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            FtBurn {
                owner_id: &sender_id,
                amount: burned_amount.into(),
                memo: None,
            }
            .emit();
        }
        used_amount.into()
    }
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "SHITZU".to_string(),
            symbol: "SHITZU".to_string(),
            icon: Some(ICON.to_string()),
            reference: None,
            reference_hash: None,
            decimals: 18,
        }
    }
}

const ICON: &str = "data:image/webp;base64,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";
