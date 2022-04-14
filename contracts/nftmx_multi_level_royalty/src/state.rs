use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use cosmwasm_std::{Addr, BlockInfo, StdResult, Storage, Decimal, Uint128};

use cw721::{ContractInfoResponse, CustomMsg, Cw721, Expiration};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex };

pub struct Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub contract_info: Item<'a, ContractInfoResponse>,
    pub minter: Item<'a, Addr>,
    pub token_count: Item<'a, u64>,
    /// Stored as (granter, operator) giving operator full control over granter's account
    pub operators: Map<'a, (&'a Addr, &'a Addr), Expiration>,
    pub tokens: IndexedMap<'a, &'a str, TokenInfo<T>, TokenIndexes<'a, T>>,
    pub(crate) _custom_response: PhantomData<C>,
}

// This is a signal, the implementations are in other files
impl<'a, T, C> Cw721<T, C> for Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
{
}

impl<T, C> Default for Cw721Contract<'static, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn default() -> Self {
        Self::new(
            "nft_info",
            "minter",
            "num_tokens",
            "operators",
            "tokens",
            "tokens__owner",
        )
    }
}

impl<'a, T, C> Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn new(
        contract_key: &'a str,
        minter_key: &'a str,
        token_count_key: &'a str,
        operator_key: &'a str,
        tokens_key: &'a str,
        tokens_owner_key: &'a str,
    ) -> Self {
        let indexes = TokenIndexes {
            owner: MultiIndex::new(token_owner_idx, tokens_key, tokens_owner_key),
        };
        Self {
            contract_info: Item::new(contract_key),
            minter: Item::new(minter_key),
            token_count: Item::new(token_count_key),
            operators: Map::new(operator_key),
            tokens: IndexedMap::new(tokens_key, indexes),
            _custom_response: PhantomData,
        }
    }

    pub fn token_count(&self, storage: &dyn Storage) -> StdResult<u64> {
        Ok(self.token_count.may_load(storage)?.unwrap_or_default())
    }

    pub fn increment_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? + 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }

    pub fn decrement_tokens(&self, storage: &mut dyn Storage) -> StdResult<u64> {
        let val = self.token_count(storage)? - 1;
        self.token_count.save(storage, &val)?;
        Ok(val)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub collection_name: String,
    pub collection_name_symbol: String,
    pub max_packable_nft: u64,
    pub max_pack_item_count: u64,
    pub max_royalyty_owner: u64,
    pub buy_sell_fee: Decimal,
    pub contract_owner: Addr
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo<T> {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Universal resource identifier for this NFT
    /// Should point to a JSON file that conforms to the ERC721
    /// Metadata JSON Schema
    pub token_uri: String,

    /// You can add any custom metadata here when you extend cw721-base
    pub extension: T,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

pub struct TokenIndexes<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub owner: MultiIndex<'a, Addr, TokenInfo<T>, Addr>,
}

impl<'a, T> IndexList<TokenInfo<T>> for TokenIndexes<'a, T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<TokenInfo<T>>> + '_> {
        let v: Vec<&dyn Index<TokenInfo<T>>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

pub fn token_owner_idx<T>(d: &TokenInfo<T>) -> Addr {
    d.owner.clone()
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PackableToken {
    pub token_id: String,
    pub token_name: String,
    pub token_uri: String,
    pub minted_by: Addr,
    pub current_owner: Addr,
    pub previous_owner: Option<Addr>,
    pub price: Uint128,
    pub number_of_transfers: Uint128,
    pub for_sale: bool
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NftPack {
    pub pack_id: u64,
    pub pack_name: String,
    pub item_count: usize,
    pub pack_items: Vec<String>,
    pub minted_by: Addr,
    pub current_owner: Addr,
    pub previous_owner: Option<Addr>,
    pub current_price: Uint128,
    pub previous_price: Uint128,
    pub number_of_transfers: u64,
    pub for_sale: bool,
    pub royalty_owners: Vec<Addr>,
    pub approvals: Vec<Addr>
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const NFTPACKCOUNTER: Item<u64> = Item::new("nft_pack_counter");

pub const ALLPACKABLENFTS: Map<&str, PackableToken> = Map::new("app_packable_nfts");
pub const TOKENURIEXISTS: Map<&str, bool> = Map::new("token_uri_exists");
pub const TOKENNAMEEXISTS: Map<&str, bool> = Map::new("token_name_exists");

pub const PACKNAMEEXISTS: Map<&str, bool> = Map::new("pack_name_exists");

pub const ROYALTYFEES: Map<(&str, &str), Decimal> = Map::new("royalty_fees");
pub const ALLNFTPACKS: Map<&str, NftPack> = Map::new("all_nft_packs");
pub const NFTPACKBALANCES: Map<&str, u64> = Map::new("nft_pack_balances");


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenPack {
    pub pack_id: u64,
    pub pack_name: String,
    pub token_address: Addr,
    pub token_amount: Uint128,
    pub minted_by: Addr,
    pub current_owner: Addr,
    pub previous_owner: Option<Addr>,
    pub current_price: Uint128,
    pub previous_price: Uint128,
    pub number_of_transfers: u64,
    pub for_sale: bool,
    pub royalty_owners: Vec<Addr>,
}

pub const ALLTOKENPACKS: Map<&str, TokenPack> = Map::new("all_token_packs");
pub const TOKENPACKCOUNTER: Item<u64> = Item::new("token_pack_counter");
pub const TOKENPACKNAMEEXISTS: Map<&str, bool> = Map::new("token_pack_name_exists");
pub const TOKENPACKBALANCES: Map<&str, u64> = Map::new("token_pack_balances");
pub const TOKENROYALTYFEES: Map<(&str, &str), Decimal> = Map::new("token_royalty_fees");

