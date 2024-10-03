use crate::*;
use near_sdk::{serde_json::json, json_types::U128, near, require, Gas, Promise, PromiseError};

const NO_ARGS: Vec<u8> = vec![];
const NO_DEPOSIT: NearToken = NearToken::from_near(0);

#[near]
impl Contract {
    pub fn get_info(&self)  {
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
    }

    // export function get_account(account_id: string): Users.User {
    //   // Returns information for the account 'account_id'
    //   if (!Users.is_registered(account_id)) {
    //     return new Users.User(u128.Zero, u128.Zero, 0, false)
    //   }

    //   const tickets: u128 = Users.get_staked_for(account_id)
    //   const unstaked: u128 = Users.get_unstaked_for(account_id)

    //   const when: u64 = Users.get_withdraw_turn_for(account_id)
    //   const now: u64 = External.get_current_turn()

    //   // Compute remaining time for withdraw to be ready
    //   const remaining: u64 = (when > now) ? when - now : 0

    //   const available: bool = unstaked > u128.Zero && now >= when

    //   return new Users.User(tickets, unstaked, remaining, available)
    // }

    // Deposit and stake ----------------------------------------------------------
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

          if internal_user_is_registered(user) {
            log!("Staking on NEW user");
            internal_add_new_user(user);
          }

        //   assert(Users.get_staked_for(user) + amount <= max_amount,
        //     `Surpassed the limit of ${max_amount} tickets that a user can have`)

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
                    json!({ "user": user, "tickets_amount": deposit})
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

        internal_stake_tickets_for(user, &tickets_amount);

        let event_args = json!({
            "standard": "nep297",
            "version": "1.0.0",
            "event": "stake_for_user",
            "data": {
                "user": &user,
                "amount": &tickets_amount,
            },
        });

        log!("EVENT_JSON:{}", event_args.to_string()) ;
    }

    // Unstake --------------------------------------------------------------------
    pub fn unstake(&mut self, amount: U128) {
        // require!(!DAO.is_emergency(), "We will be back soon");

        //   const user: string = context.predecessor
        //   assert(Users.is_registered(user), "User not registered in the pool")

        //   const user_tickets = Users.get_staked_for(user)

        //   // Check if it has enough money
        //   assert(amount <= user_tickets, "Not enough money")

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

        //   logging.log(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "unstake", "data": {"pool": "${context.contractName}", "user": "${user}", "amount": "${amount}", "all": "${withdraw_all}"}}`
        //   );

        //   return true
        // }

        // Withdraw all ---------------------------------------------------------------
        pub fn withdraw_all(&mut self) {
            //   assert(!DAO.is_emergency(), 'We will be back soon')

            //   assert(context.prepaidGas >= 20 * TGAS, "Use at least 20Tgas")

            //   const user: string = context.predecessor

            //   assert(Users.is_registered(user), "User is not registered")

            //   assert(user != DAO.get_guardian(), "The guardian cannot withdraw money")

            //   assert(External.get_current_turn() >= Users.get_withdraw_turn_for(user), "Withdraw not ready")

            //   const amount: u128 = Users.withdraw_all_for(user)
            //   assert(amount > u128.Zero, "Nothing to withdraw")

            //   // Send money to the user, always succeed
            //   ContractPromiseBatch.create(user).transfer(amount)

            //   logging.log(
            //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "transfer", "data": {"pool": "${context.contractName}", "user": "${user}", "amount": "${amount}"}}`
            //   );
            // }
        }

        // Raffle ---------------------------------------------------------------------
        // export function raffle(): string {
        //   assert(!DAO.is_emergency(), 'We will be back soon')

        //   // Function to make the raffle
        //   const now: u64 = env.block_timestamp()

        //   const next_raffle: u64 = storage.getPrimitive<u64>('nxt_raffle_tmstmp', 0)

        //   assert(now >= next_raffle, "Not enough time has passed")

        //   // Check if there is a prize to be raffled
        //   const prize: u128 = Prize.get_pool_prize()

        //   if (prize < DAO.get_min_raffle()) { return "" }

        //   // Pick a random ticket as winner
        //   const winner: string = Users.choose_random_winner()

        //   // A part goes to the reserve
        //   const fees: u128 = u128.from(DAO.get_pool_fees())
        //   const reserve_prize: u128 = (prize * fees) / u128.from(100)

        //   const guardian: string = DAO.get_guardian()
        //   Users.stake_tickets_for(guardian, reserve_prize)

        //   // We give most to the user
        //   const user_prize: u128 = prize - reserve_prize
        //   Users.stake_tickets_for(winner, user_prize)

        //   set_tickets(get_tickets() + prize)

        //   logging.log(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "prize-user", "data": {"pool": "${context.contractName}", "user": "${winner}", "amount": "${user_prize}"}}`
        //   );

        //   logging.log(
        //     `EVENT_JSON:{"standard": "nep297", "version": "1.0.0", "event": "prize-reserve", "data": {"pool": "${context.contractName}", "user": "${guardian}", "amount": "${reserve_prize}"}}`
        //   );

        //   // Set next raffle time
        //   storage.set<u64>('nxt_raffle_tmstmp', now + DAO.get_time_between_raffles())
        //   storage.set<u128>('prize', u128.Zero)

        //   winners.push(new Winner(winner, user_prize, now))
        //   return winner
        // }

        // export function number_of_winners(): i32 {
        //   // Returns the number of winners so far
        //   return winners.length
        // }

        // export function get_winners(from: u32, until: u32): Array<Winner> {
        //   assert(<i32>until <= number_of_winners(), "'until' must be <= number_of_winners")

        //   let to_return: Array<Winner> = new Array<Winner>()
        //   for (let i: i32 = <i32>from; i < <i32>until; i++) {
        //     to_return.push(winners[i])
        //   }

        //   return to_return
        // }

        // // The TOKEN contract can give part of the reserve to a user
        // export function give_from_reserve(to: string, amount: u128): void {
        //   assert(context.prepaidGas >= 120 * TGAS, "This function requires at least 120TGAS")

        //   const guardian: string = DAO.get_guardian()

        //   assert(context.predecessor == guardian, "Only the GUARDIAN can use the reserve")

        //   assert(Users.is_registered(to), "User is not registered in the pool")

        //   assert(Users.get_staked_for(guardian) >= amount, "Not enough tickets in the reserve")

        //   // Remove from reserve
        //   Users.remove_tickets_from(guardian, amount)

        //   // Give to the user, note that updating the tree can cost up to 90 TGAS
        //   Users.stake_tickets_for(to, amount)
    }
}
