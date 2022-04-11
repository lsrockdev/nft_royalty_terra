use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{ Addr };
use cw_storage_plus::{ Item, Map };
use cosmwasm_std:: { Uint256, Decimal, Timestamp, Uint128 };
use crate::asset::{Asset};
use std:: { fmt };


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct PackableNFT {
    pub token_id: String,
    pub token_name: String,
    pub token_uri: String,
    pub minted_by: Addr,
    pub current_owner: Addr,
    pub previous_owner: Addr,
    pub price: Asset,
    pub number_of_transfers: Uint128,
    pub for_sale: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct NftPack {
    pub pack_id: Uint256,
    pub pack_name: String,
    pub item_count: u64,
    pub pack_items: Vec<Uint256>,
    pub minted_by: Addr,
    pub current_owner: Addr,
    pub previous_owner: Addr,
    pub number_of_transfers: Uint128,
    pub for_sale: bool,
    pub royalty_owners: Vec<Addr>,
    //TODO royalty_fees map
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenPack {
    pub pack_id: Uint256,
    pub pack_name: String,
    pub token_address: Addr,
    pub token_amount: Uint256,
    pub minted_by: Addr,
    pub current_owner: Addr,
    pub previous_owner: Addr,
    pub current_price: Asset,
    pub previous_price: Asset,
    pub number_of_transfers: Uint128,
    pub for_sale: bool,
    pub royalty_owners: Vec<Addr>,
    //TODO royalty_fees map
}

pub const PACKABLENFTS: Map<&String, PackableNFT> = Map::new("all_packable_nfts");
pub const NFTPACKS: Map<&Uint256, NftPack> = Map::new("all_nft_packs");
pub const NFTPACKOWNERS: Map<&Uint256, Addr> = Map::new("nft_pack_owners");
pub const NFTPACKBALANCE: Map<&Addr, Uint256> = Map::new("nft_pack_balance");
pub const NFTPACKAPPROVALS: Map<&Uint256, Addr> = Map::new("nft_pack_approvals");

pub const TOKENPACKS: Map<&Uint256, TokenPack> = Map::new("token_packs");
pub const TOKENPACKOWNERS: Map<&Uint256, Addr> = Map::new("token_pack_owners");
pub const TOKENPACKBALANCE: Map<&Addr, Uint256> = Map::new("token_pack_balance");
pub const TOKENPACKAPPROVALS: Map<&Uint256, Addr> = Map::new("token_pack_approvals");

pub const TOKENAMEEXISTS: Map<&String, bool> = Map::new("token_name_exists");
pub const TOKEURIEXISTS: Map<&String, bool> = Map::new("token_uri_exists");
pub const PACKNAMEEXISTS: Map<&String, bool> = Map::new("pack_name_exists");
