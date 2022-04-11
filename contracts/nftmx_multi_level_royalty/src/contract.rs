use crate::error::ContractError;

use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_binary, DepsMut, Env, MessageInfo, Response, StdResult, Uint256, Decimal, Deps, Binary, QueryRequest, WasmQuery, Addr,
    CosmosMsg, WasmMsg, Coin, Uint128
};

use crate::msg::{ InstantiateMsg, ExecuteMsg, QueryMsg, MarketQueryMsg, MarketExecuteMsg, MigrateMsg };
use crate::asset::{Asset, AssetInfo};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
