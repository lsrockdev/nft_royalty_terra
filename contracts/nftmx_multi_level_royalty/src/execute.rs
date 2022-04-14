use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Decimal, Uint128, CosmosMsg, WasmMsg};

use cw2::set_contract_version;
use cw721::{ContractInfoResponse, CustomMsg, Cw721Execute, Cw721ReceiveMsg, Expiration};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MintMsg};
use crate::state::{
    Approval, Cw721Contract, TokenInfo, Config, CONFIG, ALLPACKABLENFTS, PackableToken, TOKENURIEXISTS,
    TOKENNAMEEXISTS, NFTPACKCOUNTER, PACKNAMEEXISTS, NftPack, ROYALTYFEES, ALLNFTPACKS, NFTPACKBALANCES,
    TokenPack, ALLTOKENPACKS, TOKENPACKCOUNTER, TOKENPACKNAMEEXISTS, TOKENPACKBALANCES, TOKENROYALTYFEES
};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use crate::asset::{Asset, AssetInfo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

impl<'a, T, C> Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
{
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response<C>> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        let info = ContractInfoResponse {
            name: msg.name.clone(),
            symbol: msg.symbol.clone(),
        };
        self.contract_info.save(deps.storage, &info)?;
        let minter = deps.api.addr_validate(&msg.minter)?;
        self.minter.save(deps.storage, &minter)?;
        let con = Config {
            collection_name: msg.name,
            collection_name_symbol: msg.symbol,
            max_packable_nft: 5000u64,
            max_pack_item_count: 10u64,
            max_royalyty_owner: 10u64,
            buy_sell_fee: Decimal::one(),
            contract_owner: deps.api.addr_validate(&msg.minter)?
        };
        CONFIG.save(deps.storage, &con)?;
        NFTPACKCOUNTER.save(deps.storage, &0u64)?;
        TOKENPACKCOUNTER.save(deps.storage, &0u64)?;
        Ok(Response::default())
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<T>,
    ) -> Result<Response<C>, ContractError> {
        match msg {
            ExecuteMsg::MintPackable(msg) => self.mint_packable(deps, env, info, msg),
            ExecuteMsg::PackNfts { token_ids, pack_name, price, royalty_fee } => 
                self.pack_nfts(deps, env, info, token_ids, pack_name, price, royalty_fee),
            ExecuteMsg::UnpackNfts { pack_id } => self.unpack_nfts(deps, env, info, pack_id),
            ExecuteMsg::ApproveNftPack { to, pack_id } => self.approve_nft_pack(deps, env, info, to, pack_id),
            ExecuteMsg::TransferNftPack { from, to, pack_id }
                => self.transfer_nft_pack(deps, env, info, from, to, pack_id),
            ExecuteMsg::PackTokens { pack_name, token_address, amount, price, royalty_fee }
                => self.pack_tokens(deps, env, info, pack_name, token_address, amount, price, royalty_fee),
            ExecuteMsg::UnpackTokens { pack_id } => self.unpack_tokens(deps, env, info, pack_id),
            ExecuteMsg::ApproveTokenPack { pack_id, to } => self.approve_token_pack(deps, env, info, pack_id, to),
            ExecuteMsg::TransferTokenPack { pack_id, from, to } => self.transfer_token_pack(deps, env, info, pack_id, from, to),
            ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => self.approve(deps, env, info, spender, token_id, expires),
            ExecuteMsg::Revoke { spender, token_id } => {
                self.revoke(deps, env, info, spender, token_id)
            }
            ExecuteMsg::ApproveAll { operator, expires } => {
                self.approve_all(deps, env, info, operator, expires)
            }
            ExecuteMsg::RevokeAll { operator } => self.revoke_all(deps, env, info, operator),
            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => self.transfer_nft(deps, env, info, recipient, token_id),
            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => self.send_nft(deps, env, info, contract, token_id, msg),
            ExecuteMsg::BurnPackable { token_id } => self.burn_packable(deps, env, info, token_id),
        }
    }
}

// TODO pull this into some sort of trait extension??
impl<'a, T, C> Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
{
    pub fn mint_packable(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: MintMsg<T>,
    ) -> Result<Response<C>, ContractError> {
        let minter = self.minter.load(deps.storage)?;
        if info.sender != minter {
            return Err(ContractError::Unauthorized {});
        }
        let token_uri_exist = TOKENURIEXISTS.load(deps.storage, &msg.token_uri)?;
        if token_uri_exist {
            return Err(ContractError::ExistTokenUri {});
        }
        let token_name_exist = TOKENNAMEEXISTS.load(deps.storage, &msg.name)?;
        if token_name_exist {
            return Err(ContractError::ExistTokenName {});
        }
        // create the token
        let token = TokenInfo {
            owner: deps.api.addr_validate(&msg.owner)?,
            approvals: vec![],
            token_uri: msg.token_uri.clone(),
            extension: msg.extension.clone()
        };
        self.tokens
            .update(deps.storage, &msg.token_id.clone(), |old| match old {
                Some(_) => Err(ContractError::Claimed {}),
                None => Ok(token),
            })?;
        self.increment_tokens(deps.storage)?;

        let packable_token = PackableToken {
            token_id: msg.token_id.clone(),
            token_name: msg.name.clone(),
            token_uri: msg.token_uri.clone(),
            minted_by: info.sender.clone(),
            current_owner: info.sender.clone(),
            previous_owner: None,
            price: msg.price.clone(),
            number_of_transfers: Uint128::zero(),
            for_sale: true
        };
        ALLPACKABLENFTS
            .update(deps.storage, &msg.token_id.clone(), |old| match old {
                Some(_) => Err(ContractError::Claimed {}),
                None => Ok(packable_token),
            })?;
        TOKENURIEXISTS.update(deps.storage, &msg.token_uri, |_| -> StdResult<_> {
            Ok(true)
        })?;
        TOKENNAMEEXISTS.update(deps.storage, &msg.name, |_| -> StdResult<_> {
            Ok(true)
        })?;
        Ok(Response::new()
            .add_attribute("action", "mint")
            .add_attribute("minter", info.sender)
            .add_attribute("token_id", msg.token_id))
    }

    pub fn burn_packable(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        let token = self.tokens.load(deps.storage, &token_id)?;        
        self.check_can_send(deps.as_ref(), &env, &info, &token)?;
        self.tokens.remove(deps.storage, &token_id)?;

        // burn packable
        let packable_token = ALLPACKABLENFTS.load(deps.storage, &token_id)?;        
        TOKENURIEXISTS.remove(deps.storage, &packable_token.token_uri);
        TOKENNAMEEXISTS.remove(deps.storage, &packable_token.token_name);
        ALLPACKABLENFTS.remove(deps.storage, &token_id);

        self.decrement_tokens(deps.storage)?;

        Ok(Response::new()
            .add_attribute("action", "burn_packable")
            .add_attribute("sender", info.sender)
            .add_attribute("token_id", token_id))
    }

    pub fn pack_nfts(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_ids: Vec<String>,
        pack_name: String,
        price: Uint128,
        royalty_fee: Decimal
    ) -> Result<Response<C>, ContractError> {
        //increment NFT Pack counter
        let pack_name_exist = PACKNAMEEXISTS.may_load(deps.storage, &pack_name)?;
        if pack_name_exist != None {
            return Err(ContractError::ExistPackName {});
        }
        let mut pack_items: Vec<String> = vec![];
        for token_id in token_ids.clone() {
            let mut token = self.tokens.load(deps.storage, &token_id)?;
            self.check_can_send(deps.as_ref(), &env, &info, &token)?;
            let packable_nft = ALLPACKABLENFTS.load(deps.storage, &token_id)?;
            if packable_nft.current_owner != info.sender {
                return Err(ContractError::NotNftOwner {});
            }
            pack_items.push(token_id.clone());

            //transfter token to this
            token.owner = env.contract.address.clone();
            token.approvals = vec![];
            self.tokens.save(deps.storage, &token_id, &token)?;
        }
        // increase pack count
        let pack_count = NFTPACKCOUNTER.load(deps.storage)? + 1;
        NFTPACKCOUNTER.save(deps.storage, &pack_count)?;

        let nft_pack = NftPack {
            pack_id: pack_count,
            pack_name: pack_name.clone(),
            item_count: token_ids.len(),
            pack_items: pack_items,
            minted_by: info.sender.clone(),
            current_owner: info.sender.clone(),
            previous_owner: None,
            current_price: price,
            previous_price: Uint128::zero(),
            number_of_transfers: 0u64,
            for_sale: true,
            royalty_owners: vec![info.sender.clone()],
            approvals: vec![]
        };
        // save all NftPack
        ALLNFTPACKS.save(deps.storage, &pack_count.to_string(), &nft_pack)?;
        //update pack name exists
        PACKNAMEEXISTS.update(deps.storage, &pack_name, |_| -> StdResult<_> {
            Ok(true)
        })?;
        //update nft pack balnace
        NFTPACKBALANCES.update(deps.storage, &info.sender.clone().to_string(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v + 1),
                None => Ok(1u64),
            }
        })?;

        ROYALTYFEES.save(deps.storage, (&pack_count.to_string(), &info.sender.clone().to_string()), &royalty_fee)?;
        Ok(Response::new()
            .add_attribute("action", "pack_nfts")
            .add_attribute("pack_id", pack_count.to_string())
            .add_attribute("pack_name", pack_name)
        )
    }

    pub fn unpack_nfts(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        pack_id: u64
    ) -> Result<Response<C>, ContractError> {
        let nft_pack_balances = NFTPACKBALANCES.load(deps.storage, &info.sender.to_string())?;
        if nft_pack_balances < 1u64 {
            return Err(ContractError::NoNftBalance {});
        }
        let nft_pack = ALLNFTPACKS.load(deps.storage, &pack_id.to_string())?;
        if nft_pack.current_owner != info.sender {
            return Err(ContractError::NotNftOwner {});
        }
        let empty = ROYALTYFEES.may_load(deps.storage, (&pack_id.to_string(), &info.sender.clone().to_string()))?;
        if empty == None {
            return Err(ContractError::NoNftPackRoyalty {});
        }
        for pack_item in nft_pack.pack_items.clone() {
            //TODO check nft token owner
            let mut token = self.tokens.load(deps.storage, &pack_item)?;
            if token.owner != env.contract.address {
                return Err(ContractError::InvalidNftOwner {});
            }
            //transfter token to sender
            token.owner = info.sender.clone();
            token.approvals = vec![];
            self.tokens.save(deps.storage, &pack_item, &token)?;
        }
        PACKNAMEEXISTS.update(deps.storage, &nft_pack.pack_name, |_| -> StdResult<_> {
            Ok(false)
        })?;
        NFTPACKBALANCES.update(deps.storage, &info.sender.clone().to_string(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v - 1),
                None => Ok(0u64),
            }
        })?;
        ALLNFTPACKS.remove(deps.storage, &pack_id.to_string());
        Ok(Response::new()
            .add_attribute("action", "unpack_nfts")
            .add_attribute("pack_id", pack_id.to_string())
        )
    }

    pub fn approve_nft_pack(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        to: String,
        pack_id: u64
    ) -> Result<Response<C>, ContractError> {
        let mut nft_pack = ALLNFTPACKS.load(deps.storage, &pack_id.to_string())?;
        if nft_pack.current_owner != info.sender {
            return Err(ContractError::NotNftOwner {});
        }
        nft_pack.approvals.push(deps.api.addr_validate(&to)?);
        ALLNFTPACKS.save(deps.storage, &pack_id.to_string(), &nft_pack)?;
        Ok(Response::new()
            .add_attribute("action", "approve_nft_pack")
            .add_attribute("to", to)
            .add_attribute("pack_id", pack_id.to_string())
        )
    }

    pub fn transfer_nft_pack(
        &self,
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        from: String,
        to: String,
        pack_id: u64
    ) -> Result<Response<C>, ContractError> {
        let mut nft_pack = ALLNFTPACKS.load(deps.storage, &pack_id.to_string())?;
        if nft_pack.current_owner.to_string() != from {
            return Err(ContractError::NotNftOwner {});
        }
        if nft_pack.approvals.len() == 0 || nft_pack.approvals.iter().find(|&x| *x == env.contract.address.clone()) == None {
            return Err(ContractError::NotNftApproved {});
        }
        nft_pack.previous_owner = Some(deps.api.addr_validate(&from)?);
        nft_pack.current_owner = deps.api.addr_validate(&to)?;
        nft_pack.previous_price = nft_pack.current_price.clone();
        nft_pack.number_of_transfers = nft_pack.number_of_transfers.clone() + 1;
        NFTPACKBALANCES.update(deps.storage, &from.clone(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v - 1),
                None => Ok(0u64),
            }
        })?;
        NFTPACKBALANCES.update(deps.storage, &to.clone(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v + 1),
                None => Ok(0u64),
            }
        })?;
        nft_pack.approvals = vec![];
        ALLNFTPACKS.save(deps.storage, &pack_id.to_string(), &nft_pack)?;
        Ok(Response::new()
            .add_attribute("action", "transfer_nft_pack")
            .add_attribute("from", from)
            .add_attribute("to", to)
            .add_attribute("pack_id", pack_id.to_string())
        )
    }

    pub fn buy_nft_pack(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        pack_id: u64
    ) -> Result<Response<C>, ContractError> {
        let nft_pack = ALLNFTPACKS.load(deps.storage, &pack_id.to_string())?;
        if nft_pack.current_owner != info.sender {
            return Err(ContractError::NotNftOwner {});
        }
        if info.funds.len() < 1 || info.funds[0].amount < nft_pack.current_price  {
            return Err(ContractError::InsufficientFunds {});
        }

        let con = CONFIG.load(deps.storage)?;
        let mut fee = info.funds[0].amount * con.buy_sell_fee;
        let mut messages: Vec<CosmosMsg> = vec![];
        let fee_asset = Asset {
            info: AssetInfo::NativeToken{ denom: "uusd".to_string()},
            amount: fee
        };
        messages.push(fee_asset.into_msg(&deps.querier, con.contract_owner.clone())?);
        if nft_pack.current_price > nft_pack.previous_price {
            for owner in nft_pack.royalty_owners {
                let royalty_fee = ROYALTYFEES.load(deps.storage, (&pack_id.to_string(), &owner.to_string()))?;
                let royalty = (nft_pack.current_price - nft_pack.previous_price) * royalty_fee;
                let royalty_asset = Asset {
                    info: AssetInfo::NativeToken{ denom: "uusd".to_string()},
                    amount: royalty
                };
                messages.push(royalty_asset.into_msg(&deps.querier, owner.clone())?);
                fee = fee + royalty;
            }
        }
        let current_owner_fee_asset = Asset {
            info: AssetInfo::NativeToken{ denom: "uusd".to_string()},
            amount: info.funds[0].amount - fee
        };
        messages.push(current_owner_fee_asset.into_msg(&deps.querier, info.sender.clone())?);

        //TODO transfer nft pack to sender
        Ok(Response::new()
            .add_attribute("action", "buy_nft_pack")
            .add_attribute("pack_id", pack_id.to_string())
        )
    }

    pub fn pack_tokens(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        pack_name: String,
        token_address: String,
        amount: Uint128,
        price: Uint128,
        royalty_fee: Decimal
    ) -> Result<Response<C>, ContractError> {
        let pack_name_exist = TOKENPACKNAMEEXISTS.may_load(deps.storage, &pack_name)?;
        if pack_name_exist != None {
            return Err(ContractError::ExistPackName {});
        }
        // TODO - transfer token to this smart contract - should be performed by user
        let pack_count = TOKENPACKCOUNTER.load(deps.storage)? + 1;
        TOKENPACKCOUNTER.save(deps.storage, &pack_count)?;

        let token_pack = TokenPack {
            pack_id: pack_count,
            pack_name: pack_name.clone(),
            token_address: deps.api.addr_validate(&token_address.clone())?,
            token_amount: amount,
            minted_by: info.sender.clone(),
            current_owner: info.sender.clone(),
            previous_owner: None,
            current_price: price,
            previous_price: Uint128::zero(),
            number_of_transfers: 0u64,
            for_sale: true,
            royalty_owners: vec![info.sender.clone()],
            approvals: vec![],
        };
        
        // update all TokenPack
        ALLTOKENPACKS.save(deps.storage, &pack_count.to_string(), &token_pack)?;
        //update pack name exists
        TOKENPACKNAMEEXISTS.update(deps.storage, &pack_name, |_| -> StdResult<_> {
            Ok(true)
        })?;
        //update token packabale balance
        TOKENPACKBALANCES.update(deps.storage, &info.sender.clone().to_string(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v + 1),
                None => Ok(1u64),
            }
        })?;
        TOKENROYALTYFEES.save(deps.storage, (&pack_count.to_string(), &info.sender.clone().to_string()), &royalty_fee)?;

        Ok(Response::new()
            .add_attribute("action", "pack_tokens")
            .add_attribute("pack_name", pack_name)
            .add_attribute("token_address", token_address)
            .add_attribute("amount", amount.to_string())
            .add_attribute("price", price.to_string())
            .add_attribute("royalty_fee", royalty_fee.to_string())
        )
    }

    pub fn unpack_tokens(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        pack_id: u64,
    ) -> Result<Response<C>, ContractError> {
        let token_pack_balances = TOKENPACKBALANCES.load(deps.storage, &info.sender.to_string())?;
        if token_pack_balances < 1u64 {
            return Err(ContractError::NoTokenBalance {});
        }
        let token_pack = ALLTOKENPACKS.load(deps.storage, &pack_id.to_string())?;
        if token_pack.current_owner != info.sender {
            return Err(ContractError::NotTokenPackOwner {});
        }
        let empty = TOKENROYALTYFEES.may_load(deps.storage, (&pack_id.to_string(), &info.sender.clone().to_string()))?;
        if empty == None {
            return Err(ContractError::NoTokenPackRoyalty {});
        }

        //TODO check token balance in this address
        let mut messages: Vec<CosmosMsg> = vec![];
        let asset = Asset {
            info: AssetInfo::Token {contract_addr: token_pack.token_address.clone().to_string()},
            amount: token_pack.token_amount
        };
        messages.push(asset.into_msg(&deps.querier, info.sender.clone())?);
        TOKENPACKNAMEEXISTS.update(deps.storage, &token_pack.pack_name, |_| -> StdResult<_> {
            Ok(false)
        })?;
        TOKENPACKBALANCES.update(deps.storage, &info.sender.clone().to_string(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v - 1),
                None => Ok(0u64),
            }
        })?;
        ALLTOKENPACKS.remove(deps.storage, &pack_id.to_string());
        Ok(Response::new()
            .add_attribute("action", "unpack_tokens")
            .add_attribute("pack_id", pack_id.to_string())
        )
    }

    pub fn approve_token_pack(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        pack_id: u64,
        to: String
    ) -> Result<Response<C>, ContractError> {
        let mut token_pack = ALLTOKENPACKS.load(deps.storage, &pack_id.to_string())?;
        if token_pack.current_owner != info.sender {
            return Err(ContractError::NotTokenPackOwner {});
        }
        token_pack.approvals.push(deps.api.addr_validate(&to)?);
        ALLTOKENPACKS.save(deps.storage, &pack_id.to_string(), &token_pack)?;
        Ok(Response::new()
            .add_attribute("action", "approve_token_pack")
            .add_attribute("pack_id", pack_id.to_string())
            .add_attribute("to", to)
        )
    }

    pub fn transfer_token_pack(
        &self,
        deps: DepsMut,
        env: Env,
        _info: MessageInfo,
        pack_id: u64,
        from: String,
        to: String
    ) -> Result<Response<C>, ContractError> {
        let mut token_pack = ALLTOKENPACKS.load(deps.storage, &pack_id.to_string())?;
        if token_pack.current_owner.to_string() != from {
            return Err(ContractError::NotTokenPackOwner {});
        }
        if token_pack.approvals.len() == 0 || token_pack.approvals.iter().find(|&x| *x == env.contract.address.clone()) == None {
            return Err(ContractError::NotTokenApproved {});
        }
        token_pack.previous_owner = Some(token_pack.current_owner.clone());
        token_pack.current_owner = deps.api.addr_validate(&to)?;
        token_pack.previous_price = token_pack.current_price.clone();
        token_pack.number_of_transfers = token_pack.number_of_transfers + 1;
        TOKENPACKBALANCES.update(deps.storage, &from.clone(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v - 1),
                None => Ok(0u64),
            }
        })?;
        TOKENPACKBALANCES.update(deps.storage, &to.clone(), |old| -> StdResult<_> {
            match old {
                Some(v) => Ok(v + 1),
                None => Ok(0u64),
            }
        })?;
        token_pack.approvals = vec![];
        ALLTOKENPACKS.save(deps.storage, &pack_id.to_string(), &token_pack)?;
        Ok(Response::new()
            .add_attribute("action", "transfer_token_pack")
            .add_attribute("pack_id", pack_id.to_string())
            .add_attribute("from", from)
            .add_attribute("to", to)
        )
    }
}

impl<'a, T, C> Cw721Execute<T, C> for Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
{
    type Err = ContractError;

    fn transfer_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        self._transfer_nft(deps, &env, &info, &recipient, &token_id)?;

        Ok(Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", recipient)
            .add_attribute("token_id", token_id))
    }

    fn send_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response<C>, ContractError> {
        // Transfer token
        self._transfer_nft(deps, &env, &info, &contract, &token_id)?;

        let send = Cw721ReceiveMsg {
            sender: info.sender.to_string(),
            token_id: token_id.clone(),
            msg,
        };

        // Send message
        Ok(Response::new()
            .add_message(send.into_cosmos_msg(contract.clone())?)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", info.sender)
            .add_attribute("recipient", contract)
            .add_attribute("token_id", token_id))
    }

    fn approve(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> Result<Response<C>, ContractError> {
        self._update_approvals(deps, &env, &info, &spender, &token_id, true, expires)?;

        Ok(Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", info.sender)
            .add_attribute("spender", spender)
            .add_attribute("token_id", token_id))
    }

    fn revoke(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        self._update_approvals(deps, &env, &info, &spender, &token_id, false, None)?;

        Ok(Response::new()
            .add_attribute("action", "revoke")
            .add_attribute("sender", info.sender)
            .add_attribute("spender", spender)
            .add_attribute("token_id", token_id))
    }

    fn approve_all(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response<C>, ContractError> {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(ContractError::Expired {});
        }

        // set the operator for us
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.operators
            .save(deps.storage, (&info.sender, &operator_addr), &expires)?;

        Ok(Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", info.sender)
            .add_attribute("operator", operator))
    }

    fn revoke_all(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        operator: String,
    ) -> Result<Response<C>, ContractError> {
        let operator_addr = deps.api.addr_validate(&operator)?;
        self.operators
            .remove(deps.storage, (&info.sender, &operator_addr));

        Ok(Response::new()
            .add_attribute("action", "revoke_all")
            .add_attribute("sender", info.sender)
            .add_attribute("operator", operator))
    }

    fn burn(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response<C>, ContractError> {
        let token = self.tokens.load(deps.storage, &token_id)?;
        self.check_can_send(deps.as_ref(), &env, &info, &token)?;

        self.tokens.remove(deps.storage, &token_id)?;
        self.decrement_tokens(deps.storage)?;

        Ok(Response::new()
            .add_attribute("action", "burn")
            .add_attribute("sender", info.sender)
            .add_attribute("token_id", token_id))
    }
}

// helpers
impl<'a, T, C> Cw721Contract<'a, T, C>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
{
    pub fn _transfer_nft(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        recipient: &str,
        token_id: &str,
    ) -> Result<TokenInfo<T>, ContractError> {
        let mut token = self.tokens.load(deps.storage, token_id)?;
        // ensure we have permissions
        self.check_can_send(deps.as_ref(), env, info, &token)?;
        // set owner and remove existing approvals
        token.owner = deps.api.addr_validate(recipient)?;
        token.approvals = vec![];
        self.tokens.save(deps.storage, token_id, &token)?;
        Ok(token)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn _update_approvals(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        spender: &str,
        token_id: &str,
        // if add == false, remove. if add == true, remove then set with this expiration
        add: bool,
        expires: Option<Expiration>,
    ) -> Result<TokenInfo<T>, ContractError> {
        let mut token = self.tokens.load(deps.storage, token_id)?;
        // ensure we have permissions
        self.check_can_approve(deps.as_ref(), env, info, &token)?;

        // update the approval list (remove any for the same spender before adding)
        let spender_addr = deps.api.addr_validate(spender)?;
        token.approvals = token
            .approvals
            .into_iter()
            .filter(|apr| apr.spender != spender_addr)
            .collect();

        // only difference between approve and revoke
        if add {
            // reject expired data as invalid
            let expires = expires.unwrap_or_default();
            if expires.is_expired(&env.block) {
                return Err(ContractError::Expired {});
            }
            let approval = Approval {
                spender: spender_addr,
                expires,
            };
            token.approvals.push(approval);
        }

        self.tokens.save(deps.storage, token_id, &token)?;

        Ok(token)
    }

    /// returns true iff the sender can execute approve or reject on the contract
    pub fn check_can_approve(
        &self,
        deps: Deps,
        env: &Env,
        info: &MessageInfo,
        token: &TokenInfo<T>,
    ) -> Result<(), ContractError> {
        // owner can approve
        if token.owner == info.sender {
            return Ok(());
        }
        // operator can approve
        let op = self
            .operators
            .may_load(deps.storage, (&token.owner, &info.sender))?;
        match op {
            Some(ex) => {
                if ex.is_expired(&env.block) {
                    Err(ContractError::Unauthorized {})
                } else {
                    Ok(())
                }
            }
            None => Err(ContractError::Unauthorized {}),
        }
    }

    /// returns true iff the sender can transfer ownership of the token
    pub fn check_can_send(
        &self,
        deps: Deps,
        env: &Env,
        info: &MessageInfo,
        token: &TokenInfo<T>,
    ) -> Result<(), ContractError> {
        // owner can send
        if token.owner == info.sender {
            return Ok(());
        }

        // any non-expired token approval can send
        if token
            .approvals
            .iter()
            .any(|apr| apr.spender == info.sender && !apr.is_expired(&env.block))
        {
            return Ok(());
        }

        // operator can send
        let op = self
            .operators
            .may_load(deps.storage, (&token.owner, &info.sender))?;
        match op {
            Some(ex) => {
                if ex.is_expired(&env.block) {
                    Err(ContractError::Unauthorized {})
                } else {
                    Ok(())
                }
            }
            None => Err(ContractError::Unauthorized {}),
        }
    }
}
