use chrono::Utc;
use near_primitives::types::AccountId;
use near_sdk::json_types::U128;
use near_sdk::NearToken;
use near_workspaces::network::Sandbox;
use near_workspaces::{Account, Contract, Worker};
use poolparty::pool::Pool;
use poolparty::UserInfo;
use serde_json::json;

pub async fn init(
) -> Result<(Account, Account, Account, Contract, Worker<Sandbox>), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;

    let root = sandbox.root_account().unwrap();

    // who controls the reserve
    let guardian = root
        .create_subaccount("guardian")
        .initial_balance(NearToken::from_near(20))
        .transact()
        .await?
        .unwrap();

    // the pool party contract
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // the mock validator
    let staking_contract = sandbox
        .dev_deploy(&std::fs::read("./tests/mock-validator/validator.wasm")?)
        .await?;

    let now = Utc::now().timestamp();
    let one_minute = 60 * 1000000000;
    let a_minute_from_now = now * 1000000000 + one_minute;

    let init = contract
        .call("new")
        .args_json(json!(
            {
                "guardian": guardian.id(),
                "external_pool": staking_contract.id(),
                "first_raffle": a_minute_from_now.to_string(),
                "time_between_raffles": one_minute.to_string(),
            }
        ))
        .transact()
        .await?;

    assert!(init.is_success());

    let guardian_deposit = guardian
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    assert!(guardian_deposit.is_success());

    let ana = root
        .create_subaccount("ana")
        .initial_balance(NearToken::from_near(60))
        .transact()
        .await?
        .unwrap();

    let bob = root
        .create_subaccount("bob")
        .initial_balance(NearToken::from_near(20))
        .transact()
        .await?
        .unwrap();

    Ok((ana, bob, guardian, contract, sandbox))
}

// Pool -----------------------------------------------------------
#[tokio::test]
async fn test_deposit() -> Result<(), Box<dyn std::error::Error>> {
    let (ana, bob, guardian, contract, _sandbox) = init().await?;

    let ana_deposit = ana
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(50))
        .max_gas()
        .transact()
        .await?;

    let bob_deposit = bob
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    assert!(ana_deposit.is_success());
    assert!(bob_deposit.is_success());

    let ana_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": ana.id()}))
        .await?
        .json::<UserInfo>()?;
    assert_eq!(
        ana_balance.staked.as_yoctonear(),
        NearToken::from_near(50).as_yoctonear()
    );

    let bob_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": bob.id()}))
        .await?
        .json::<UserInfo>()?;
    assert_eq!(
        bob_balance.staked.as_yoctonear(),
        NearToken::from_near(1).as_yoctonear()
    );

    let guardian_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": guardian.id()}))
        .await?
        .json::<UserInfo>()?;

    assert_eq!(
        guardian_balance.staked.as_yoctonear(),
        NearToken::from_near(1).as_yoctonear()
    );

    let pool_info = contract.view("get_pool_info").await?.json::<Pool>()?;

    assert_eq!(
        pool_info.tickets.as_yoctonear(),
        ana_balance
            .staked
            .saturating_add(bob_balance.staked)
            .saturating_add(guardian_balance.staked)
            .as_yoctonear()
    );

    Ok(())
}

// Emergency -----------------------------------------------------------
#[tokio::test]
async fn test_emergency() -> Result<(), Box<dyn std::error::Error>> {
    let (ana, _bob, _guardian, contract, _sandbox) = init().await?;

    // User can't start or stop emergency
    let user_emergency_start = ana
        .call(contract.id(), "emergency_start")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(user_emergency_start.is_failure());

    let user_emergency_stop = ana
        .call(contract.id(), "emergency_stop")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(user_emergency_stop.is_failure());

    // Contract can start emergency
    let contract_emergency_start = contract
        .call("emergency_start")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(contract_emergency_start.is_success());

    // User can't deposit or unstake during emergency
    let deposit_during_emergency = ana
        .call(contract.id(), "deposit_and_stake")
        .args_json(json!({}))
        .deposit(NearToken::from_near(2))
        .max_gas()
        .transact()
        .await?;
    assert!(deposit_during_emergency.is_failure());

    let unstake_during_emergency = ana
        .call(contract.id(), "unstake")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?;
    assert!(unstake_during_emergency.is_failure());

    // User can't raffle during emergency
    let raffle_during_emergency = ana
        .call(contract.id(), "raffle")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?;
    assert!(raffle_during_emergency.is_failure());

    // User can't withdraw during emergency
    let withdraw_during_emergency = ana
        .call(contract.id(), "withdraw_all")
        .args_json(json!({}))
        .max_gas()
        .transact()
        .await?;
    assert!(withdraw_during_emergency.is_failure());

    // Contract can stop emergency
    let contract_emergency_stop = contract
        .call("emergency_stop")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(contract_emergency_stop.is_success());

    Ok(())
}

#[tokio::test]
async fn eucliden_div() -> Result<(), Box<dyn std::error::Error>> {
    let a = 10u128;

    assert_eq!(a.div_euclid(9u128), 1);
    assert_eq!(a.div_euclid(6u128), 1);
    assert_eq!(a.div_euclid(5u128), 2);
    assert_eq!(a.div_euclid(3u128), 3);
    return Ok(());
}

#[tokio::test]
async fn test_random_u128() -> Result<(), Box<dyn std::error::Error>> {
    let (_ana, _bob, _guardian, contract, _sandbox) = init().await?;
    println!("Running test_random_u128, which may take a while... please wait!");

    let twenty_five_near = NearToken::from_near(25).as_yoctonear();

    let tries = 100;
    let mut results = vec![0, 0, 0, 0, 0];

    let min = twenty_five_near;
    let max = twenty_five_near.saturating_mul(5);

    for _ in 0..tries {
        let rand_u128 = contract
            .call("random_u128")
            .args_json(json!((min.to_string(), max.to_string())))
            .max_gas()
            .transact()
            .await?
            .json::<U128>()?;
        assert!(rand_u128.0 >= min && rand_u128.0 < max);

        results[(rand_u128.0.div_euclid(twenty_five_near)) as usize] += 1;
    }

    for i in 1..=4 {
        let count = results[i];
        assert!(
            count >= 15 && count <= 35, // 99% confidence interval
            "Number {} appeared {} times",
            i,
            count
        );
    }

    return Ok(());
}

#[tokio::test]
async fn test_raffle() -> Result<(), Box<dyn std::error::Error>> {
    let (ana, bob, _guardian, contract, sandbox) = init().await?;

    let _ana_deposit = ana
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(50))
        .max_gas()
        .transact()
        .await?;

    let _bob_deposit = bob
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    // Fast forward 200 blocks
    let blocks_to_advance = 200;

    sandbox.fast_forward(blocks_to_advance).await?;
    // Before the raffle
    let pool_info_before = contract.view("get_pool_info").await?.json::<Pool>()?;

    let prize_update_outcome = ana
        .call(contract.id(), "update_prize")
        .max_gas()
        .transact()
        .await?;
    assert!(prize_update_outcome.is_success());

    let prize = contract.view("get_pool_info").await?.json::<Pool>()?.prize;
    let pool_fee = pool_info_before.pool_fee;
    let reserve_prize = (prize.as_yoctonear() * pool_fee as u128) / 100u128;

    let user_prize = NearToken::from_yoctonear(prize.as_yoctonear() - reserve_prize);

    let ana_before_win = contract
        .view("get_user_info")
        .args_json(json!({"user": ana.id()}))
        .await?
        .json::<UserInfo>()?;

    let raffle = contract.call("raffle").max_gas().transact().await?;
    let winner = raffle.json::<AccountId>()?;

    assert_eq!(&winner, ana.id());

    // After the raffle
    let ana_after_win = contract
        .view("get_user_info")
        .args_json(json!({"user": ana.id()}))
        .await?
        .json::<UserInfo>()?;

    let pool_info_after = contract.view("get_pool_info").await?.json::<Pool>()?;

    assert_eq!(
        ana_after_win.staked,
        (ana_before_win.staked.saturating_add(user_prize))
    );
    assert_eq!(
        pool_info_after.tickets,
        pool_info_before.tickets.saturating_add(prize)
    );
    assert_eq!(pool_info_after.prize, NearToken::from_yoctonear(0));

    Ok(())
}

#[tokio::test]
async fn test_unstake_and_withdraw() -> Result<(), Box<dyn std::error::Error>> {
    let (ana, bob, _guardian, contract, _sandbox) = init().await?;

    let _ana_deposit = ana
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(50))
        .max_gas()
        .transact()
        .await?;

    let _bob_deposit = bob
        .call(contract.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(1))
        .max_gas()
        .transact()
        .await?;

    let ana_unstake = ana
        .call(contract.id(), "unstake")
        .args_json(json!({"amount": NearToken::from_near(10)}))
        .max_gas()
        .transact()
        .await?;
    assert!(ana_unstake.is_success());

    let ana_balance = contract
        .view("get_user_info")
        .args_json(json!({"user": ana.id()}))
        .await?
        .json::<UserInfo>()?;

    assert_eq!(
        ana_balance.staked.as_yoctonear(),
        NearToken::from_near(40).as_yoctonear()
    );
    assert_eq!(ana_balance.available, NearToken::from_near(10));
    assert_eq!(ana_balance.withdraw_turn.0, 1);

    let pool_info = contract.view("get_pool_info").await?.json::<Pool>()?;
    assert_eq!(
        pool_info.to_unstake.as_yoctonear(),
        NearToken::from_near(10).as_yoctonear()
    );

    let interact_external = contract
        .call("interact_external")
        .max_gas()
        .transact()
        .await?;
    assert!(interact_external.is_success());

    let pool_info = contract.view("get_pool_info").await?.json::<Pool>()?;
    assert_eq!(pool_info.to_unstake.as_yoctonear(), 0);
    assert_eq!(pool_info.next_withdraw_turn, 2);
    assert_eq!(
        pool_info.tickets.as_yoctonear(),
        NearToken::from_near(42).as_yoctonear()
    );

    let _bob_unstake = bob
        .call(contract.id(), "unstake")
        .args_json(json!({"amount": NearToken::from_near(1)}))
        .max_gas()
        .transact()
        .await?;

    let bob_details = contract
        .view("get_user_info")
        .args_json(json!({"user": bob.id()}))
        .await?
        .json::<UserInfo>()?;   
    assert_eq!(bob_details.withdraw_turn.0, 2);

    let ana_prev = ana.view_account().await?;
    //  Ana doesnt wait the 4 epochs
    let ana_withdraw = ana
        .call(contract.id(), "withdraw_all")
        .max_gas()
        .transact()
        .await?;
    assert!(ana_withdraw.is_failure());

    // Ana waits the 4 epochs
    // TODO

    let ana_current = ana.view_account().await?;

    // Round up Annas balances
    let roundup_prev = roundup_balance(ana_prev.balance);
    let roundup_curr = roundup_balance(ana_current.balance);
    // assert_eq!(
    //     roundup_prev + NearToken::from_near(10).as_yoctonear(),
    //     roundup_curr
    // );

    let pool_info = contract.view("get_pool_info").await?.json::<Pool>()?;
    assert_eq!(pool_info.next_withdraw_turn, 2);

    Ok(())
}

// Helpers --------------------------------------------------------
fn roundup_balance(amount: NearToken) -> u128 {
    let rem = amount.as_yoctonear() % 10u128.pow(24);
    amount.as_yoctonear() - rem + 10u128.pow(24)
}
