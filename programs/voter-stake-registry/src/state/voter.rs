use crate::{error::*, state::deposit_entry::DepositEntry};
use anchor_lang::prelude::*;

/// User account for minting voting rights.
#[account(zero_copy)]
pub struct Voter {
    pub deposits: [DepositEntry; 32],
    pub voter_authority: Pubkey,
    pub registrar: Pubkey,
    pub voter_bump: u8,
    pub voter_weight_record_bump: u8,
    pub _reserved1: [u8; 14],
}
const_assert!(std::mem::size_of::<Voter>() == 136 * 32 + 32 + 32 + 1 + 1 + 14);
const_assert!(std::mem::size_of::<Voter>() % 8 == 0);

impl Voter {
    pub const MIN_OWN_WEIGHTED_STAKE: u64 = 15_000_000;

    /// The full vote weight available to the voter
    pub fn weight(&self) -> Result<u64> {
        self.deposits
            .iter()
            .filter(|d| d.is_used)
            .try_fold(0_u64, |sum, d| {
                d.voting_power().map(|vp| sum.checked_add(vp).unwrap())
            })
    }

    /// The vote weight available to the voter when ignoring any lockup effects
    pub fn weight_baseline(&self) -> u64 {
        self.deposits
            .iter()
            .filter(|d| d.is_used)
            .fold(0, |acc, d| acc + d.amount_deposited_native)
    }

    /// The extra lockup vote weight that the user is guaranteed to have at `at_ts`, assuming
    /// they withdraw and unlock as much as possible starting from `curr_ts`.
    pub fn weight_locked_guaranteed(&self, curr_ts: i64, at_ts: i64) -> Result<u64> {
        require_gte!(at_ts, curr_ts, MplStakingError::InvalidTimestampArguments);
        self.deposits
            .iter()
            .filter(|d| d.is_used)
            .try_fold(0_u64, |sum, _d| Ok(sum))
    }

    pub fn active_deposit_mut(&mut self, index: u8) -> Result<&mut DepositEntry> {
        let index = index as usize;
        require_gt!(
            self.deposits.len(),
            index,
            MplStakingError::OutOfBoundsDepositEntryIndex
        );
        let d = &mut self.deposits[index];
        require!(d.is_used, MplStakingError::UnusedDepositEntryIndex);
        Ok(d)
    }

    pub fn active_deposit(&self, index: u8) -> Result<&DepositEntry> {
        let index = index as usize;
        require_gt!(
            self.deposits.len(),
            index,
            MplStakingError::OutOfBoundsDepositEntryIndex
        );
        let d = &self.deposits[index];
        require!(d.is_used, MplStakingError::UnusedDepositEntryIndex);
        Ok(d)
    }
}

#[macro_export]
macro_rules! voter_seeds {
    ( $voter:expr ) => {
        &[
            $voter.registrar.as_ref(),
            b"voter".as_ref(),
            $voter.voter_authority.as_ref(),
            &[$voter.voter_bump],
        ]
    };
}

use static_assertions::const_assert;
pub use voter_seeds;
