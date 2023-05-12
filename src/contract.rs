#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::error::ContractError::{AllPending, InvalidParameters};
use crate::msg::{ExecuteMsg, GetJobIdResponse, InstantiateMsg, PalomaMsg, QueryMsg};
use crate::state::{State, RETRY_DELAY, STATE, WITHDRAW_TIMESTAMP};
use cosmwasm_std::CosmosMsg;
use ethabi::{Contract, Function, Param, ParamType, StateMutability, Token, Uint};
use std::collections::BTreeMap;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:limit-order-bot-univ2-cw";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        job_id: msg.job_id.clone(),
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;
    RETRY_DELAY.save(deps.storage, &msg.retry_delay)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("job_id", msg.job_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<PalomaMsg>, ContractError> {
    match msg {
        ExecuteMsg::PutWithdraw {
            deposit_ids,
            profit_taking_or_stop_loss,
        } => execute::withdraw(deps, env, deposit_ids, profit_taking_or_stop_loss),
    }
}

pub mod execute {
    use super::*;

    pub fn withdraw(
        deps: DepsMut,
        env: Env,
        deposit_ids: Vec<u32>,
        profit_taking_or_stop_loss: Vec<bool>,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
        if deposit_ids.len() != profit_taking_or_stop_loss.len() {
            return Err(InvalidParameters {});
        }
        #[allow(deprecated)]
        let contract: Contract = Contract {
            constructor: None,
            functions: BTreeMap::from_iter(vec![(
                "multiple_withdraw".to_string(),
                vec![Function {
                    name: "multiple_withdraw".to_string(),
                    inputs: vec![
                        Param {
                            name: "deposit_ids".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Uint(256))),
                            internal_type: None,
                        },
                        Param {
                            name: "profit_taking_or_stop_loss".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Bool)),
                            internal_type: None,
                        },
                    ],
                    outputs: Vec::new(),
                    constant: None,
                    state_mutability: StateMutability::NonPayable,
                }],
            )]),
            events: BTreeMap::new(),
            errors: BTreeMap::new(),
            receive: false,
            fallback: false,
        };

        let mut tokens_id: Vec<Token> = vec![];
        let mut tokens_take_profit: Vec<Token> = vec![];
        let retry_delay: u64 = RETRY_DELAY.load(deps.storage)?;
        let mut i = 0;
        while i < deposit_ids.len() {
            let deposit_id = deposit_ids[i];
            let profit_taking = profit_taking_or_stop_loss[i];
            if let Some(timestamp) = WITHDRAW_TIMESTAMP.may_load(deps.storage, deposit_id)? {
                if timestamp.plus_seconds(retry_delay).lt(&env.block.time) {
                    tokens_id.push(Token::Uint(Uint::from_big_endian(
                        &deposit_id.to_be_bytes(),
                    )));
                    tokens_take_profit.push(Token::Bool(profit_taking));
                    WITHDRAW_TIMESTAMP.save(deps.storage, deposit_id, &env.block.time)?;
                }
            } else {
                tokens_id.push(Token::Uint(Uint::from_big_endian(
                    &deposit_id.to_be_bytes(),
                )));
                tokens_take_profit.push(Token::Bool(profit_taking));
                WITHDRAW_TIMESTAMP.save(deps.storage, deposit_id, &env.block.time)?;
            }
            i += 1;
        }

        if tokens_id.is_empty() {
            Err(AllPending {})
        } else {
            let tokens = vec![Token::Array(tokens_id), Token::Array(tokens_take_profit)];
            Ok(Response::new()
                .add_message(CosmosMsg::Custom(PalomaMsg {
                    job_id: state.job_id,
                    payload: Binary(
                        contract
                            .function("multiple_withdraw")
                            .unwrap()
                            .encode_input(tokens.as_slice())
                            .unwrap(),
                    ),
                }))
                .add_attribute("action", "multiple_withdraw"))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetJobId {} => to_binary(&query::get_job_id(deps)?),
    }
}

pub mod query {
    use super::*;

    pub fn get_job_id(deps: Deps) -> StdResult<GetJobIdResponse> {
        let state = STATE.load(deps.storage)?;
        Ok(GetJobIdResponse {
            job_id: state.job_id,
        })
    }
}
