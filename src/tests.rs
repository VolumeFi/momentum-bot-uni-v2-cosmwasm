//! Smoke tests.

use crate::contract::{execute, instantiate};
use crate::msg::{Deposit, ExecuteMsg, InstantiateMsg};
use crate::ContractError;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::Uint256;

/// Test instantiating the contract, creating a pool, adding liquidity and making a trade.
#[test]
fn happy_path() -> Result<(), ContractError> {
    let mut deps = mock_dependencies();
    let mut env = mock_env();

    let info = mock_info("admin0000", &[]);
    let _ = instantiate(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        InstantiateMsg {
            retry_delay: 60,
            job_id: "test_job".to_string(),
        },
    )?;

    let r = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::PutWithdraw {
            deposits: vec![
                Deposit {
                    deposit_id: 0u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
                Deposit {
                    deposit_id: 1u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
                Deposit {
                    deposit_id: 2u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
            ],
        },
    )?;

    assert_eq!(r.messages.len(), 1);
    let r = execute(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        ExecuteMsg::PutWithdraw {
            deposits: vec![
                Deposit {
                    deposit_id: 0u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
                Deposit {
                    deposit_id: 1u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
                Deposit {
                    deposit_id: 2u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
            ],
        },
    )
    .is_err();
    assert!(r);

    env.block.time = env.block.time.plus_seconds(100u64);

    let r = execute(
        deps.as_mut(),
        env,
        info.clone(),
        ExecuteMsg::PutWithdraw {
            deposits: vec![
                Deposit {
                    deposit_id: 0u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
                Deposit {
                    deposit_id: 1u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
                Deposit {
                    deposit_id: 2u32,
                    min_amount0: Uint256::zero(),
                    withdraw_type: 0,
                },
            ],
        },
    )?;
    assert_eq!(r.messages.len(), 1);

    Ok(())
}
