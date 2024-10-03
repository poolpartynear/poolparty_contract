use crate::*;
use near_sdk::{near, NearToken};

#[near(serializers=[borsh, json])]
pub struct User {
    staked_balance: NearToken,
    unstaked_balance: NearToken,
    available_when: u64,
    available: bool,
}

#[near]
impl Contract {
    pub(crate) fn internal_user_is_registered(&self, user: &AccountId) -> bool {
        true
    }
    fn get_user_uid() {}
    fn get_staked_for() {}
    fn get_unstaked_for() {}
    fn get_withdraw_turn_for() {}
    fn get_total_users() {}
    fn take_over_guardia() {}
    fn set_withdraw_turn() {}
    pub(crate) fn internal_add_new_user() {}

    fn remove_tickets_from() {}
    fn unstake_tickets_for() {}
    fn withdraw_all_for() {}
    fn random_u128() {}
    fn choose_random_winner() {}
    fn find_user_with_ticket() {}


    pub(crate) fn internal_stake_tickets_for(&mut self, user: AccountId, amount: NearToken) {
    
    }
}
S
    pub fn internal_user_is_registered(user: &AccountId) -> bool {
        true
    }
