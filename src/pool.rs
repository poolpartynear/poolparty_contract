use crate::*;
use near_sdk::{near, require, serde_json::json, Gas, Promise, PromiseError};
use users::Winner;

const NO_ARGS: Vec<u8> = vec![];
const NO_DEPOSIT: NearToken = NearToken::from_near(0);

#[near]
impl Contract {
    pub fn get_pool_info(&self) -> Pool {
        self.pool.clone()
        // Returns the: amount of tickets in the pool, current prize,
        // next timestamp to do the raffle, and if we should call the external pool
        // const to_unstake: u128 = External.get_to_unstake()
        // const tickets: u128 = get_tickets() - to_unstake
        // const next_raffle: u64 = storage.getPrimitive<u64>('nxt_raffle_tmstmp', 0)
        // const prize: u128 = Prize.get_pool_prize()
        // const fees: u8 = DAO.get_pool_fees()
        // const last_prize_update: u64 = Prize.get_last_prize_update()

        // const reserve: u128 = Users.get_staked_for(DAO.get_guardian())

        // const withdraw_external_ready: bool = External.can_withdraw_external()

        // return new PoolInfo(tickets, to_unstake, reserve, prize, fees, last_prize_update,
        //                     next_raffle, withdraw_external_ready)
        // }
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
        // Todo: evaluate if we need this log
        if self.user_storage.is_registered(&user) {
            log!("Staking on EXISTING user");
        }

        require!(
            self.user_storage
                .get_staked_for(&user)
                .saturating_add(deposit_amount)
                .le(&self.config.max_deposit),
            format!(
                "Surpassed the limit of {} tickets that a user can have",
                &self.config.max_deposit
            )
        );

        // Deposit the tokens in the external pool

        // Add the tickets to the pool, but not yet to the user (rollback if failed)
        self.pool.pool_tickets.saturating_add(deposit_amount);

        // Todo: check validity - We add 100yn to cover the cost of staking in an external pool
        let deposit = env::attached_deposit(); // might need + 100yn;

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
        #[callback_result] call_result: Result<NearToken, PromiseError>,
        user: AccountId,
        tickets_amount: NearToken,
    ) {
        // It failed, remove tickets from the pool and return the tokens to the user
        if call_result.is_err() {
            self.pool.pool_tickets.saturating_sub(tickets_amount);

            log!("Failed attempt to deposit in the pool, returning tokens to the user");
            Promise::new(user.clone()).transfer(tickets_amount);
        }

        // It worked, give tickets to the user
        self.user_storage.stake_tickets_for(&user, tickets_amount);

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

    // Unstake --------------------------------------------------------------------
    pub fn unstake(&mut self, user: AccountId, amount: NearToken) {
        require!(!self.config.emergency, "We will be back soon");
        require!(
            self.user_storage.is_registered(&user),
            "User not registered in the pool"
        );

        let user_tickets = self.user_storage.get_staked_for(&user);

        require!(
            amount.le(&user_tickets),
            format!("Amount cant exceed {}", user_tickets)
        );

        //   const withdraw_all: bool = (user_tickets - amount) < DAO.get_min_deposit();
        //   if (withdraw_all) {
        //     amount = user_tickets
        //   }

        //   // add to the amount we will unstake from external next time
        //   External.set_to_unstake(External.get_to_unstake() + amount)

        //   // the user will be able to withdraw in the next withdraw_turn
        //   Users.set_withdraw_turn(user, External.get_next_withdraw_turn())

        //   // update user info
        //   Users.unstake_tickets_for(user, amount)
        //
        //

        let event_args = json!({
            "standard": "nep297",
            "version": "1.0.0",
            "event": "unstake",
            "data": {
                "user": user,
                "amount": amount,
                // "all": withdraw_all,
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
        require!(
            self.user_storage.is_registered(&user),
            "User is not registered"
        );

        //   assert(External.get_current_turn() >= Users.get_withdraw_turn_for(user), "Withdraw not ready")

        let amount: NearToken = self.user_storage.withdraw_all_for(&user);
        require!(!amount.is_zero(), "Nothing to withdraw");

        // Tranfer the tokens to the user
        Promise::new(user.clone()).transfer(amount);

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
        let winner: AccountId = self.user_storage.choose_random_winner();

        // A part goes to the reserve
        //   const fees: u128 = u128.from(DAO.get_pool_fees())
        //   const reserve_prize: u128 = (prize * fees) / u128.from(100)

        //   const guardian: string = DAO.get_guardian()
        //   Users.stake_tickets_for(guardian, reserve_prize)

        // We give most to the user
        //   const user_prize: u128 = prize - reserve_prize
        //   Users.stake_tickets_for(winner, user_prize)

        //   set_tickets(get_tickets() + prize)

        //   logging.log(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "prize-user", "data": {"pool": "${context.contractName}", "user": "${winner}", "amount": "${user_prize}"}}`
        //   );

        //   logging.log(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "prize-reserve", "data": {"pool": "${context.contractName}", "user": "${guardian}", "amount": "${reserve_prize}"}}`
        //   );

        // Set next raffle time
        self.pool.next_raffle = now + self.config.time_between_raffles;
        self.pool.prize_pool = NearToken::from_near(0);

        self.pool.winners.push(Winner(winner.clone(), prize, now));

        winner
    }

    pub fn number_of_winners(&self) -> u32 {
        self.pool.winners.len() as u32
    }

    pub fn get_winners(&self, from: u32, limit: u32) -> Vec<&Winner> {
        require!(
            from.lt(&self.number_of_winners()),
            format!("'from' must be < {}", self.number_of_winners())
        );

        require!(
            limit.gt(&0) && limit.le(&self.number_of_winners()),
            format!("'limit' must be between 1 and {}", self.number_of_winners())
        );

        self.pool
            .winners
            .iter()
            .skip(from as usize)
            .take(limit as usize)
            .collect()
    }
}
