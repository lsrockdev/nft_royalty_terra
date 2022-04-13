use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Decimal, Uint128};

use cw2::set_contract_version;
use cw721::{ContractInfoResponse, CustomMsg, Cw721Execute, Cw721ReceiveMsg, Expiration};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MintMsg};
use crate::state::{
    Approval, Cw721Contract, TokenInfo, Config, CONFIG, ALLPACKABLENFTS, PackableToken, TOKENURIEXISTS,
    TOKENNAMEEXISTS, NFTPACKCOUNTER, PACKNAMEEXISTS, NftPack, ROYALTYFEES, ALLNFTPACKS
};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};

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
            buy_sell_fee: Decimal::one()
        };
        CONFIG.save(deps.storage, &con)?;
        NFTPACKCOUNTER.save(deps.storage, &0u64)?;
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
            // extension: msg.extension,
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
        let pack_name_exist = PACKNAMEEXISTS.load(deps.storage, &pack_name)?;
        if pack_name_exist {
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
        let pack_count = NFTPACKCOUNTER.load(deps.storage)? + 1;
        // NFTPACKCOUNTER.save(deps.storage, &val)?;

        // let nft_pack = NftPack {
        //     pack_id: pack_count,
        //     pack_name: pack_name,
        //     item_count: token_ids.len(),
        //     pack_items: pack_items,
        //     minted_by: info.sender.clone(),
        //     current_owner: info.sender.clone(),
        //     previous_owner: None,
        //     current_price: price,
        //     previous_price: Uint128::zero(),
        //     number_of_transfers: 0u64,
        //     for_sale: true,
        //     royalty_owners: vec![]
        // };
        // ALLNFTPACKS.save(deps.storage, &pack_count, &nft_pack)?;
        NFTPACKCOUNTER.save(deps.storage, &pack_count)?;

        Ok(Response::new()
            .add_attribute("action", "burn_packable")
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
