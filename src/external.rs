use crate::*;
use near_sdk::{require, serde_json::json, Gas, Promise, PromiseError};

#[near]
impl Contract {
    // Semaphore to interact with external pool
    pub fn is_interacting(&self) -> bool {
        self.pool.is_interacting
    }

    pub fn start_interacting(&mut self) {
        require!(
            !self.is_interacting(),
            "Already interacting with the staking contract"
        );

        self.pool.is_interacting = true;
    }

    pub fn stop_interacting(&mut self) {
        self.pool.is_interacting = false;
    }

    // TODO
    // pub fn get_current_turn(&self)-> u64 {
    //   // The current_turn increases by 1 each time we withdraw from external
    //   return storage.getPrimitive<u64>('current_turn', 0)
    // }

    pub fn get_next_withdraw_turn(&self) -> u64 {
        // The withdraw_turn increases by 1 each time we unstake from external.
        // When a user unstakes, we asign them a withdraw turn. The user can
        // withdraw when current_turn is equal to their asigned turn
        self.pool.next_withdraw_turn
    }

    pub fn get_next_withdraw_epoch(&self) -> u64 {
        self.pool.next_withdraw_epoch
    }

    pub fn can_withdraw_external(&self) -> bool {
        if env::epoch_height() >= self.pool.next_withdraw_epoch {
            return true;
        } else {
            return false;
        }
    }

    // Unstake external -----------------------------------------------------------
    pub fn unstake_external(&mut self) -> Promise {
        require!(env::prepaid_gas() >= Gas::from_tgas(300), "Not enough gas"); // Todo: evaluate gas

        require!(
            self.pool.to_unstake > NearToken::from_yoctonear(0),
            "Nothing to unstake!"
        );
        // Check if we are already interacting
        self.start_interacting();

        // TODO: If someone wants to unstake, they will get the next turn
        //  self.users.(user, External.get_next_withdraw_turn());

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "unstake".to_string(),
                json!({ "amount": self.pool.to_unstake.as_yoctonear()})
                    .to_string()
                    .into_bytes(),
                NO_DEPOSIT,
                Gas::from_tgas(120),
            )
            .then(
                Promise::new(env::current_account_id()).function_call(
                    "unstake_external_callback".to_string(),
                    json!({ "amount": self.pool.to_unstake})
                        .to_string()
                        .into_bytes(),
                    NO_DEPOSIT,
                    Gas::from_tgas(45), // Todo: Check the Gas amount
                ),
            )
    }

    #[private]
    pub fn unstake_external_callback(
        &mut self,
        amount: NearToken,
        #[callback_result] call_result: Result<NearToken, PromiseError>,
    ) {
        if call_result.is_err() {
            // Rollback next_withdraw_turn
            self.pool.next_withdraw_turn -= 1;
        } else {
            self.pool.pool_tickets.saturating_sub(amount);
            self.pool.next_withdraw_epoch = env::epoch_height() + self.config.epochs_wait;
            // TODO: Increase the turn?
            // TODO: Ask for bellow
            // next time we want to withdraw
            //     storage.set<string>('external_action', 'withdraw')

            self.pool.to_unstake = self.pool.to_unstake.saturating_sub(amount);
        }
        self.stop_interacting();
    }

    // Withdraw external ----------------------------------------------------------
    pub fn withdraw_external(&mut self) -> Promise {
        require!(env::prepaid_gas() >= Gas::from_tgas(300), "Not enough gas"); // TODO: evaluate

        // Check that 4 epochs passed from the last unstake from external
        require!(
            env::epoch_height() >= self.pool.next_withdraw_epoch,
            "Not enough time has passed"
        );

        // Check if we are already interacting, if not, set it to true()
        self.start_interacting();

        // Withdraw tokens from external pool

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "withdraw_all".to_string(),
                NO_ARGS,
                NO_DEPOSIT,
                Gas::from_tgas(120), // Todo: Check the Gas amount
            )
            .then(Promise::new(env::current_account_id()).function_call(
                "withdraw_external_callback".to_string(),
                NO_ARGS,
                NO_DEPOSIT,
                Gas::from_tgas(120), // Todo: Check the Gas amount
            ))
    }

    #[private]
    pub fn withdraw_external_callback(
        &mut self,
        #[callback_result] call_result: Result<(), PromiseError>,
    ) -> bool {
        self.stop_interacting();

        if call_result.is_err() {
            // Rollback next_withdraw_epoch
            self.pool.next_withdraw_epoch -= 1;
            false
        } else {
            // TODO ASK
            //  storage.set<string>('external_action', 'unstake')
            self.pool.next_withdraw_turn += 1;
            true
        }
    }
}
