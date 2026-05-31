#![no_std]

mod errors;
#[cfg(test)]
mod test;

use errors::CrowdfundError;
use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, token, Address, Env};

#[contracttype]
enum DataKey {
    Organizer,
    Token,
    Goal,
    Deadline,
    Raised,
}

#[contract]
pub struct CrowdfundContract;

#[contractimpl]
impl CrowdfundContract {
    /// Initialise a campaign. Sets the funding goal (in token base units)
    /// and the deadline (Unix timestamp after which no contributions are
    /// accepted). Only callable once.
    ///
    /// # Arguments
    /// * `organizer` – address that will receive funds if the goal is met.
    /// * `token`     – accepted payment token.
    /// * `goal`      – target amount in token base units (must be > 0).
    /// * `deadline`  – Unix timestamp of the campaign end (must be in the future).
    pub fn init_campaign(
        env: Env,
        organizer: Address,
        token: Address,
        goal: i128,
        deadline: u64,
    ) {
        if env.storage().persistent().has(&DataKey::Organizer) {
            panic_with_error!(&env, CrowdfundError::AlreadyInitialized);
        }
        if goal <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidGoal);
        }
        if deadline <= env.ledger().timestamp() {
            panic_with_error!(&env, CrowdfundError::InvalidDeadline);
        }

        env.storage().persistent().set(&DataKey::Organizer, &organizer);
        env.storage().persistent().set(&DataKey::Token, &token);
        env.storage().persistent().set(&DataKey::Goal, &goal);
        env.storage().persistent().set(&DataKey::Deadline, &deadline);
        env.storage().persistent().set(&DataKey::Raised, &0_i128);
    }

    /// Contribute `amount` tokens to the campaign. The caller must have
    /// pre-approved the contract to spend at least `amount` from their
    /// balance. Panics after the deadline or if the campaign is not yet
    /// initialised.
    pub fn contribute(env: Env, contributor: Address, amount: i128) {
        contributor.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, CrowdfundError::InvalidAmount);
        }

        let deadline: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        if env.ledger().timestamp() > deadline {
            panic_with_error!(&env, CrowdfundError::CampaignEnded);
        }

        let token_addr: Address = env
            .storage()
            .persistent()
            .get(&DataKey::Token)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));

        let contract_addr = env.current_contract_address();
        token::TokenClient::new(&env, &token_addr)
            .transfer(&contributor, &contract_addr, &amount);

        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&DataKey::Raised, &raised.saturating_add(amount));
    }

    // ── Read-only accessors ───────────────────────────────────────────────────

    pub fn goal(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized))
    }

    pub fn deadline(env: Env) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::Deadline)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized))
    }

    pub fn raised(env: Env) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0)
    }

    pub fn organizer(env: Env) -> Address {
        env.storage()
            .persistent()
            .get(&DataKey::Organizer)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized))
    }

    /// Returns `true` when the raised amount has reached or exceeded the goal.
    pub fn goal_reached(env: Env) -> bool {
        let goal: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Goal)
            .unwrap_or_else(|| panic_with_error!(&env, CrowdfundError::NotInitialized));
        let raised: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::Raised)
            .unwrap_or(0);
        raised >= goal
    }
}
