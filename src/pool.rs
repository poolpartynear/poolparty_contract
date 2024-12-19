use crate::*;
use near_sdk::{json_types::U128, near, require, serde_json::json, Gas, Promise, PromiseError};

// Amount of time between prize updates (10 sec)
// To avoid blocking the interaction with external pool
const PRIZE_UPDATE_INTERVAL: u64 = 10000000000;

#[near(serializers=[json])]
pub struct ExternalUser {
    unstaked_balance: NearToken,
    staked_balance: NearToken,
    available: bool,
}

#[near(serializers=[borsh, json])]
#[derive(Clone, Debug)]
pub struct Pool {
    pub to_unstake: NearToken,
    pub prize: NearToken,
    pub last_prize_update: u64,
    pub pool_fee: u8,
    pub next_raffle: u64,
    pub tickets: NearToken,
    pub is_interacting: bool,
    pub next_withdraw_turn: u64,
    pub next_withdraw_epoch: u64,
}

impl Pool {
    pub(crate) fn new(first_raffle: u64) -> Self {
        Self {
            tickets: NearToken::from_yoctonear(0),
            to_unstake: NearToken::from_yoctonear(0),
            prize: NearToken::from_yoctonear(0),
            last_prize_update: 0,
            pool_fee: 0,
            next_raffle: first_raffle,
            is_interacting: false,
            next_withdraw_turn: 1,
            next_withdraw_epoch: 0,
        }
    }
}

#[near]
impl Contract {
    #[payable]
    pub fn deposit_and_stake(&mut self) -> Promise {
        require!(!self.config.emergency, "We will be back soon");

        let tickets = env::attached_deposit();

        require!(
            tickets.ge(&self.config.min_deposit),
            format!(
                "Please attach at least {}",
                &self.config.min_deposit.exact_amount_display()
            )
        );

        let user = env::predecessor_account_id();

        if self.users.tree.len() == 0 {
            require!(
                user == self.config.guardian,
                "Only the guardian can deposit first"
            );
        }

        if !self.is_registered(&user) {
            self.add_new_user(&user);
        }

        require!(
            self.get_staked_for(&user) + tickets.as_yoctonear()
                <= self.config.max_deposit.as_yoctonear(),
            format!(
                "Surpassed the limit of {} tickets that a user can have",
                &self.config.max_deposit
            )
        );

        // Deposit the tokens in the external pool
        // Add the tickets to the pool, but not yet to the user (rollback if failed)
        self.pool.tickets = self.pool.tickets.saturating_add(tickets);

        // Todo: check validity - We add 100yn to cover the cost of staking in an external pool
        let deposit = env::attached_deposit().saturating_add(NearToken::from_yoctonear(100));

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "deposit_and_stake".to_string(),
                NO_ARGS,
                deposit,
                Gas::from_tgas(12),
            )
            .then(
                Promise::new(env::current_account_id()).function_call(
                    "deposit_and_stake_callback".to_string(),
                    json!({"user": user, "tickets_amount": tickets})
                        .to_string()
                        .into_bytes(),
                    NO_DEPOSIT,
                    Gas::from_tgas(45),
                ),
            )
    }

    #[private]
    pub fn deposit_and_stake_callback(
        &mut self,
        #[callback_result] call_result: Result<(), PromiseError>,
        user: AccountId,
        tickets_amount: NearToken,
    ) {
        // It failed, remove tickets from the pool and return the tokens to the user
        if call_result.is_err() {
            self.pool.tickets = self.pool.tickets.saturating_sub(tickets_amount);

            log!("Failed attempt to deposit in the pool, returning tokens to the user");
            Promise::new(user.clone()).transfer(tickets_amount);
        } else {
            // It worked, give tickets to the user
            self.stake_tickets_for(&user, tickets_amount.as_yoctonear());

            let event_args = json!({
                "standard": "nep297",
                "version": "1.0.0",
                "event": "stake_for_user",
                "data": {
                    "user": &user,
                    "amount": &tickets_amount,
                },
            });

            log!("EVENT_JSON:{}", event_args.to_string());
        }
    }

    // Unstake --------------------------------------------------------------------
    pub fn unstake(&mut self, amount: NearToken) {
        let user = env::predecessor_account_id();

        require!(!self.config.emergency, "We will be back soon");
        require!(self.is_registered(&user), "User not registered in the pool");

        let user_tickets = self.get_staked_for(&user);

        require!(
            amount.as_yoctonear() <= user_tickets,
            format!("Amount cant exceed {}", user_tickets)
        );

        let mut unstake_amount = amount;

        let withdraw_all: bool =
            (user_tickets - amount.as_yoctonear()) < self.config.min_deposit.as_yoctonear();
        if withdraw_all {
            unstake_amount = NearToken::from_yoctonear(user_tickets);
        }

        // add to the amount we will unstake from external next time
        self.pool.to_unstake = self.pool.to_unstake.saturating_add(amount);

        // the user will be able to withdraw in the next withdraw_turn
        self.set_withdraw_turn_for(&user, self.pool.next_withdraw_turn);

        // update user info
        self.unstake_tickets_for(&user, amount);

        let event_args = json!({
            "standard": "nep297",
            "version": "1.0.0",
            "event": "unstake",
            "data": {
                "user": user,
                "amount": unstake_amount,
                "all": withdraw_all,
            },
        });

        log!("EVENT_JSON:{}", event_args.to_string());
    }

    // Withdraw all ---------------------------------------------------------------
    pub fn withdraw_all(&mut self) {
        let user = env::predecessor_account_id();

        require!(!self.config.emergency, "We will be back soon");
        require!(
            env::prepaid_gas().ge(&Gas::from_tgas(20)),
            "Use at least 20Tgas"
        );
        require!(self.is_registered(&user), "User is not registered");

        require!(
            self.pool.next_withdraw_turn >= self.get_withdraw_turn_for(&user).unwrap(),
            "Withdraw not ready"
        );

        let amount = self.withdraw_all_for(&user);
        require!(amount != 0, "Nothing to withdraw");

        require!(
            env::epoch_height() >= self.pool.next_withdraw_epoch,
            "Not enough time has passed"
        );

        // Transfer the tokens to the user
        Promise::new(user.clone()).transfer(NearToken::from_yoctonear(amount));

        let event_args = json!({
            "standard": "nep297",
            "version": "1.0.0",
            "event": "transfer",
            "data": {
                "user": user,
                "amount": U128(amount),
            },
        });

        log!("EVENT_JSON:{}", event_args.to_string());
    }

    // Raffle ---------------------------------------------------------------------
    pub fn raffle(&mut self) -> AccountId {
        require!(!self.config.emergency, "We will be back soon");
        require!(!self.users.tree.len() > 3, "No users in the pool");

        let now: u64 = env::block_timestamp_ms();
        let prize: NearToken = self.pool.prize;

        require!(now.ge(&self.pool.next_raffle), "Not enough time has passed");
        require!(
            prize.ge(&self.config.min_to_raffle),
            "Not enough prize to raffle"
        );

        // Pick a random ticket as winner
        let winner: AccountId = self.choose_random_winner();

        // Part goes to the reserve via pool_fee
        let guardian = self.config.guardian.clone();
        let pool_fee = (prize.as_yoctonear() * self.pool.pool_fee as u128) / 100u128;
        self.stake_tickets_for(&guardian, pool_fee);

        // Give the prize to the winner (minus the pool_fee)
        self.stake_tickets_for(&winner, prize.as_yoctonear() - pool_fee);

        // add the prize to the pool, and reset the prize_pool
        self.pool.tickets = self.pool.tickets.saturating_add(prize);
        
        // Set next raffle time
        self.pool.next_raffle = now + self.config.time_between_raffles;
        self.pool.prize = NearToken::from_near(0);

        winner
    }

    pub fn update_prize(&mut self) -> Promise {
        require!(!self.config.emergency, "We will be back soon");

        require!(
            env::prepaid_gas().gt(&Gas::from_tgas(40)),
            "Please use at least 40Tgas"
        );

        let now: u64 = env::block_timestamp_ms();
        let last_update: u64 = self.pool.last_prize_update;

        log!(
            "Last update: {}\n now: {}",
            last_update + PRIZE_UPDATE_INTERVAL,
            now
        );

        require!(
            now.ge(&(last_update + PRIZE_UPDATE_INTERVAL)),
            "Not enough time has passed"
        );

        // Block interaction with external pool
        self.start_interacting();

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "get_account".to_string(),
                json!({ "account_id": env::current_account_id()})
                    .to_string()
                    .into_bytes(),
                NO_DEPOSIT,
                Gas::from_tgas(20), // Todo: Check the Gas amount
            )
            .then(Promise::new(env::current_account_id()).function_call(
                "update_prize_callback".to_string(),
                NO_ARGS,
                NO_DEPOSIT,
                Gas::from_tgas(20), // Todo: Check the Gas amount
            ))
    }

    #[private]
    pub fn update_prize_callback(
        &mut self,
        #[callback_result] call_result: Result<ExternalUser, PromiseError>,
    ) -> NearToken {
        // Unblock interaction with external pool
        self.stop_interacting();

        let mut prize: NearToken = self.pool.prize;

        if call_result.is_err() {
            // Todo: emit event?
            log!("Failed to update the prize");
            return prize;
        }

        let staked_in_external: NearToken = call_result.unwrap().staked_balance;

        // The difference between the staked_balance in the external pool and the
        // tickets we have in our pool is the prize
        if staked_in_external.gt(&self.pool.tickets) {
            prize = staked_in_external.saturating_sub(self.pool.tickets);
        }

        // Update prize_pool
        self.pool.prize = prize.min(self.config.max_to_raffle);

        // Update last_prize_update
        self.pool.last_prize_update = env::block_timestamp_ms();

        prize
    }

    #[private]
    pub fn set_pool_fee(&mut self, fee: u8) {
        self.pool.pool_fee = fee;
    }
}
