#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::error::ContractError::AllPending;
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
        ExecuteMsg::PutWithdraw { deposits } => execute::withdraw(deps, env, deposits),
    }
}

pub mod execute {
    use super::*;
    use crate::msg::Deposit;

    pub fn withdraw(
        deps: DepsMut,
        env: Env,
        deposits: Vec<Deposit>,
    ) -> Result<Response<PalomaMsg>, ContractError> {
        let state = STATE.load(deps.storage)?;
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
                            name: "min_amounts0".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Uint(256))),
                            internal_type: None,
                        },
                        Param {
                            name: "withdraw_types".to_string(),
                            kind: ParamType::Array(Box::new(ParamType::Uint(256))),
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
        let mut tokens_min_amount: Vec<Token> = vec![];
        let mut tokens_withdraw_type: Vec<Token> = vec![];
        let retry_delay: u64 = RETRY_DELAY.load(deps.storage)?;
        for deposit in deposits {
            let deposit_id = deposit.deposit_id;
            let min_amount = deposit.min_amount0;
            let withdraw_type = deposit.withdraw_type;
            if let Some(timestamp) = WITHDRAW_TIMESTAMP.may_load(deps.storage, deposit_id)? {
                if timestamp.plus_seconds(retry_delay).lt(&env.block.time) {
                    tokens_id.push(Token::Uint(Uint::from_big_endian(
                        &deposit_id.to_be_bytes(),
                    )));
                    tokens_min_amount.push(Token::Uint(Uint::from_big_endian(
                        &min_amount.to_be_bytes(),
                    )));
                    tokens_withdraw_type.push(Token::Uint(Uint::from_big_endian(
                        &withdraw_type.to_be_bytes(),
                    )));
                    WITHDRAW_TIMESTAMP.save(deps.storage, deposit_id, &env.block.time)?;
                }
            } else {
                tokens_id.push(Token::Uint(Uint::from_big_endian(
                    &deposit_id.to_be_bytes(),
                )));
                tokens_min_amount.push(Token::Uint(Uint::from_big_endian(
                    &min_amount.to_be_bytes(),
                )));
                tokens_withdraw_type.push(Token::Uint(Uint::from_big_endian(
                    &withdraw_type.to_be_bytes(),
                )));
                WITHDRAW_TIMESTAMP.save(deps.storage, deposit_id, &env.block.time)?;
            }
        }

        if tokens_id.is_empty() {
            Err(AllPending {})
        } else {
            let tokens = vec![
                Token::Array(tokens_id),
                Token::Array(tokens_min_amount),
                Token::Array(tokens_withdraw_type),
            ];
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
