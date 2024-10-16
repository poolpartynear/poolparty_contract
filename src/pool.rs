use crate::*;
use near_sdk::{json_types::U128, near, require, serde_json::json, Gas, Promise, PromiseError};
use users::Winner;

const PRIZE_UPDATE_INTERVAL: u64 = 10000000000;

#[near(serializers=[borsh, json])]
#[derive(Clone)]
pub struct Pool {
    pub total_staked: NearToken,
    pub to_unstake: NearToken,
    pub reserve: NearToken,
    pub prize_pool: NearToken,
    pub last_prize_update: u64,
    pub next_raffle: u64,
    pub withdraw_ready: bool,
    pub pool_tickets: NearToken,
    pub total_users: u64,
    pub winners: Vec<Winner>,
    pub is_interacting: bool,
    pub next_withdraw_turn: u64,
    pub next_withdraw_epoch: u64,
}

impl Default for Pool {
    fn default() -> Self {
        Self {
            total_staked: NearToken::from_yoctonear(0),
            to_unstake: NearToken::from_yoctonear(0),
            reserve: NearToken::from_yoctonear(0),
            prize_pool: NearToken::from_yoctonear(0),
            last_prize_update: 0,
            next_raffle: 0,
            withdraw_ready: false,
            pool_tickets: NearToken::from_yoctonear(0),
            total_users: 0,
            winners: vec![],
            is_interacting: false,
            next_withdraw_turn: 0,
            next_withdraw_epoch: 0,
        }
    }
}

#[near]
impl Contract {
    pub fn get_pool_info(&self) -> &Pool {
        &self.pool
    }

    pub fn get_number_of_winners(&self) -> u32 {
        self.pool.winners.len() as u32
    }

    pub fn get_winners(&self, from: Option<u32>, limit: Option<u32>) -> Vec<&Winner> {
        let from = from.unwrap_or(0);
        let limit = limit.unwrap_or(10);

        self.pool
            .winners
            .iter()
            .skip(from as usize)
            .take(limit as usize)
            .collect()
    }

    pub fn get_last_prize_update(&self) -> u64 {
        self.pool.last_prize_update
    }

    pub fn get_pool_prize(&self) -> NearToken {
        self.pool.prize_pool
    }

    #[payable]
    pub fn deposit_and_stake(&mut self) -> Promise {
        let deposit_amount = env::attached_deposit();

        require!(!self.config.emergency, "We will be back soon");
        require!(
            deposit_amount.ge(&self.config.min_deposit),
            format!(
                "Please attach at least {}",
                &self.config.min_deposit.exact_amount_display()
            )
        );

        let user = env::predecessor_account_id();

        if !self.users.is_registered(&user) {
            self.users.add_new_user(&user);
        }

        require!(
            self.users.get_staked_for(&user) + deposit_amount.as_yoctonear()
                <= self.config.max_deposit.as_yoctonear(),
            format!(
                "Surpassed the limit of {} tickets that a user can have",
                &self.config.max_deposit
            )
        );

        // Deposit the tokens in the external pool

        // Add the tickets to the pool, but not yet to the user (rollback if failed)
        self.pool.pool_tickets = self.pool.pool_tickets.saturating_add(deposit_amount);

        // Todo: check validity - We add 100yn to cover the cost of staking in an external pool
        let deposit = env::attached_deposit().saturating_add(NearToken::from_yoctonear(100)); // might need + 100yn;

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "deposit_and_stake".to_string(),
                NO_ARGS,
                deposit,
                Gas::from_tgas(12), // Todo: Check the Gas amount
            )
            .then(
                Promise::new(env::current_account_id()).function_call(
                    "deposit_and_stake_callback".to_string(),
                    json!({ "user": user, "tickets_amount": deposit_amount})
                        .to_string()
                        .into_bytes(),
                    NO_DEPOSIT,
                    Gas::from_tgas(45), // Todo: Check the Gas amount
                ),
            )
    }

    #[private]
    pub fn deposit_and_stake_callback(
        &mut self,
        #[callback_result] call_result: Result<U128, PromiseError>,
        user: AccountId,
        tickets_amount: NearToken,
    ) {
        // It failed, remove tickets from the pool and return the tokens to the user
        if call_result.is_err() {
            self.pool.pool_tickets = self.pool.pool_tickets.saturating_sub(tickets_amount);

            log!("Failed attempt to deposit in the pool, returning tokens to the user");
            Promise::new(user.clone()).transfer(tickets_amount);
        } else {
            self.users
                .stake_tickets_for(&user, tickets_amount.as_yoctonear());

            // It worked, give tickets to the user

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
    pub fn unstake(&mut self, user: AccountId, amount: NearToken) {
        require!(!self.config.emergency, "We will be back soon");
        require!(
            self.users.is_registered(&user),
            "User not registered in the pool"
        );

        let user_tickets = self.users.get_staked_for(&user);
        let mut unstake_amount = amount;

        require!(
            unstake_amount.as_yoctonear() <= user_tickets,
            format!("Amount cant exceed {}", user_tickets)
        );

        let withdraw_all: bool =
            (user_tickets - amount.as_yoctonear()) < self.config.min_deposit.as_yoctonear();
        if withdraw_all {
            unstake_amount = NearToken::from_yoctonear(user_tickets);
        }

        // add to the amount we will unstake from external next time
        self.pool.to_unstake.saturating_add(amount);

        //   // the user will be able to withdraw in the next withdraw_turn
        //   Users.set_withdraw_turn(user, External.get_next_withdraw_turn())

        // update user info
        self.users.unstake_tickets_for(&user, amount);

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
        ); // Todo: Check the Gas amount
        require!(self.users.is_registered(&user), "User is not registered");

        //   assert(External.get_current_turn() >= Users.get_withdraw_turn_for(user), "Withdraw not ready")

        let amount = self.users.withdraw_all_for(&user);
        require!(amount != 0, "Nothing to withdraw");

        // Transfer the tokens to the user
        Promise::new(user.clone()).transfer(NearToken::from_yoctonear(amount));

        let event_args = json!({
            "standard": "nep297",
            "version": "1.0.0",
            "event": "transfer",
            "data": {
                "user": user,
                "amount": amount,
            },
        });

        log!("EVENT_JSON:{}", event_args.to_string());
    }

    // Raffle ---------------------------------------------------------------------
    pub fn raffle(&mut self) -> AccountId {
        require!(!self.config.emergency, "We will be back soon");

        let now: u64 = env::block_timestamp_ms();
        let prize: NearToken = self.pool.prize_pool;

        require!(now.ge(&self.pool.next_raffle), "Not enough time has passed");
        require!(
            prize.ge(&self.config.min_to_raffle),
            "Not enough prize to raffle"
        );

        // Pick a random ticket as winner
        let winner: AccountId = self.users.choose_random_winner();

        self.users.stake_tickets_for(&winner, prize.as_yoctonear());
        self.pool.pool_tickets.saturating_add(prize);

        // TODO: Emit events
        //  log!(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "prize-user", "data": {"pool": "${context.contractName}", "user": "${winner}", "amount": "${user_prize}"}}`
        //   );

        //  log!(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "prize-reserve", "data": {"pool": "${context.contractName}", "user": "${guardian}", "amount": "${reserve_prize}"}}`
        //   );

        // Set next raffle time
        self.pool.next_raffle = now + self.config.time_between_raffles;
        self.pool.prize_pool = NearToken::from_near(0);

        self.pool.winners.push(Winner(winner.clone(), prize, now));

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

        require!(
            now.ge(&(last_update + PRIZE_UPDATE_INTERVAL)),
            "Not enough time has passed"
        );

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "get_account_staked_balance".to_string(),
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
        #[callback_result] call_result: Result<NearToken, PromiseError>,
    ) -> NearToken {
        let mut prize: NearToken = self.pool.prize_pool;

        if call_result.is_err() {
            // Todo: emit event?
            log!("Failed to update the prize");
        }

        let staked_in_external: NearToken = call_result.unwrap();

        // The difference between the staked_balance in the external pool and the
        // tickets we have in our pool is the prize
        if staked_in_external.gt(&self.pool.pool_tickets) {
            prize = staked_in_external.saturating_sub(self.pool.pool_tickets);
        }

        if prize.gt(&self.config.max_to_raffle) {
            prize = self.config.max_to_raffle
        }

        // Todo: emit event

        // Update prize_pool
        log!("New prize: {}", prize.exact_amount_display());
        self.pool.prize_pool = prize;

        // Update last_prize_update
        self.pool.last_prize_update = env::block_timestamp_ms();

        prize
    }
}
