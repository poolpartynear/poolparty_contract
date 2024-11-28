use crate::*;
use near_sdk::{require, serde_json::json, Gas, Promise, PromiseError};

#[near]
impl Contract {
    // Semaphore to interact with external pool
    pub(crate) fn start_interacting(&mut self) {
        require!(
            !self.pool.is_interacting,
            "Already interacting with the staking contract"
        );

        self.pool.is_interacting = true;
    }

    pub(crate) fn stop_interacting(&mut self) {
        self.pool.is_interacting = false;
    }

    // Interact with external pool ------------------------------------------------
    pub fn interact_external(&mut self) -> Promise {
        require!(!self.config.emergency, "We'll be back soon");

        match self.next_action {
            Action::Unstake => self.unstake_external(),
            Action::Withdraw => self.withdraw_external(),
        }
    }

    // Unstake external -----------------------------------------------------------
    fn unstake_external(&mut self) -> Promise {
        require!(env::prepaid_gas() >= Gas::from_tgas(300), "Not enough gas"); // Todo: evaluate gas

        require!(
            self.pool.to_unstake > NearToken::from_yoctonear(0),
            "Nothing to unstake!"
        );

        // Check if we are already interacting
        self.start_interacting();

        self.pool.next_withdraw_turn += 1;

        Promise::new(self.config.external_pool.clone())
            .function_call(
                "unstake".to_string(),
                json!({ "amount": self.pool.to_unstake})
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
        #[callback_result] call_result: Result<(), PromiseError>,
    ) {
        if call_result.is_err() {
            log!("Error while unstaking from external pool");
            // Rollback next_withdraw_turn
            self.pool.next_withdraw_turn -= 1;
        } else {
            self.pool.tickets = self.pool.tickets.saturating_sub(amount);
            self.pool.next_withdraw_epoch = env::epoch_height() + self.config.epochs_wait;

            // next time we want to withdraw
            self.next_action = Action::Withdraw;

            self.pool.to_unstake = self.pool.to_unstake.saturating_sub(amount);
        }
        self.stop_interacting();
    }

    // Withdraw external ----------------------------------------------------------
    fn withdraw_external(&mut self) -> Promise {
        require!(env::prepaid_gas() >= Gas::from_tgas(300), "Not enough gas"); // TODO: evaluate

        // Check that 6 epochs passed from the last unstake from external
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
            self.next_action = Action::Unstake;
            self.pool.next_withdraw_turn += 1;
            true
        }
    }
}
