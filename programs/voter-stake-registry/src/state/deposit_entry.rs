use crate::state::{
    lockup::{Lockup, LockupKind},
    LockupPeriod,
};
use anchor_lang::prelude::*;
use static_assertions::const_assert;

/// Bookkeeping for a single deposit for a given mint and lockup schedule.
#[zero_copy]
#[derive(Default, Debug)]
pub struct DepositEntry {
    // Locked state.
    pub lockup: Lockup,
    /// Delegated staker. It's an address of a Delegate.
    pub delegate: Pubkey,
    /// Amount in deposited, in native currency. Withdraws of vested tokens
    /// directly reduce this amount.
    /// This directly tracks the total amount added by the user. They may
    /// never withdraw more than this amount.
    pub amount_deposited_native: u64,
    /// The last time when the delegate was updated
    pub delegate_last_update_ts: u64,
    // Points to the VotingMintConfig this deposit uses.
    pub voting_mint_config_idx: u8,
    // True if the deposit entry is being used.
    pub is_used: bool,
    pub _reserved0: [u8; 32],
    pub _reserved1: [u8; 6],
}
const_assert!(std::mem::size_of::<DepositEntry>() == 48 + 32 + 8 + 8 + 1 + 1 + 32 + 6);
const_assert!(std::mem::size_of::<DepositEntry>() % 8 == 0);

impl DepositEntry {
    /// # Voting Power Caclulation
    /// ### Constant Lockup
    /// Voting Power will always be equal to 1*deposited
    /// since we don't provide any other methods besides constant locking
    pub fn voting_power(&self) -> Result<u64> {
        Ok(self.amount_deposited_native)
    }

    /// Returns native tokens still locked.
    #[inline(always)]
    pub fn amount_locked(&self) -> u64 {
        if self.is_staked() {
            self.amount_deposited_native
        } else {
            0
        }
    }

    /// Returns native tokens that are unlocked given current vesting
    /// and previous withdraws.
    #[inline(always)]
    pub fn amount_unlocked(&self) -> u64 {
        if self.is_staked() {
            0
        } else {
            self.amount_deposited_native
        }
    }

    /// Returns the weighted stake for the given deposit at the specified timestamp.
    #[inline(always)]
    pub fn weighted_stake(&self, curr_ts: u64) -> u64 {
        if !self.is_staked() {
            return 0;
        }

        self.lockup.multiplier(curr_ts) * self.amount_deposited_native
    }

    /// Weighted stake can be calculated only if `DepositEntry` is active,
    ///  and if both `LockupKind` and `LockupPeriod` not `None`.
    #[inline(always)]
    fn is_staked(&self) -> bool {
        self.is_used
            && self.lockup.kind.ne(&LockupKind::None)
            && self.lockup.period.ne(&LockupPeriod::None)
            && !self.lockup.cooldown_requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        LockupKind::{Constant, None},
        LockupPeriod,
    };

    #[test]
    pub fn far_future_lockup_start_test() -> Result<()> {
        // Check that voting power stays correct even if the lockup is very far in the
        // future, or at least more than lockup_saturation_secs in the future.
        let lockup_start = 10_000_000_000; // arbitrary point
        let period = LockupPeriod::Flex;
        let delegate = Pubkey::new_unique();
        let deposit = DepositEntry {
            amount_deposited_native: 20_000,
            delegate,
            lockup: Lockup {
                start_ts: lockup_start,
                end_ts: lockup_start + LockupPeriod::Flex.to_secs(), // start + cooldown + period
                kind: Constant,
                period,
                ..Default::default()
            },
            is_used: true,
            ..Default::default()
        };

        let baseline_vote_weight = deposit.amount_deposited_native;
        assert_eq!(baseline_vote_weight, 20_000);

        // The timestamp 100_000 is very far before the lockup_start timestamp
        let withdrawable = deposit.amount_unlocked();
        assert_eq!(withdrawable, 0);

        let voting_power = deposit.voting_power().unwrap();
        assert_eq!(voting_power, 20_000);

        Ok(())
    }

    #[test]
    fn test_weighted_stake_unused() {
        let deposit = DepositEntry {
            amount_deposited_native: 20_000,
            ..Default::default()
        };
        assert_eq!(deposit.weighted_stake(0), 0);
    }

    #[test]
    fn test_weighted_stake_expired() {
        let amount = 20_000;
        let deposit = DepositEntry {
            amount_deposited_native: amount,
            lockup: Lockup::default(),
            is_used: true,
            ..Default::default()
        };
        assert_eq!(deposit.weighted_stake(10), amount);
    }

    #[test]
    fn test_weighted_stake_under_cooldown() {
        let amount = 20_000;
        let deposit = DepositEntry {
            amount_deposited_native: amount,
            lockup: Lockup {
                end_ts: 100,
                cooldown_requested: true,
                ..Default::default()
            },
            is_used: true,
            ..Default::default()
        };
        assert_eq!(deposit.weighted_stake(150), 0);
    }

    #[test]
    fn test_weighted_stake() {
        let amount = 20_000;
        let deposit = DepositEntry {
            amount_deposited_native: amount,
            lockup: Lockup {
                end_ts: 100,
                kind: Constant,
                period: LockupPeriod::OneYear,
                ..Default::default()
            },
            is_used: true,
            ..Default::default()
        };
        assert_eq!(
            deposit.weighted_stake(50),
            amount * LockupPeriod::OneYear.multiplier()
        );
    }

    #[test]
    fn test_weighted_stake_is_invalid() {
        let amount = 20_000;

        let deposit = DepositEntry {
            amount_deposited_native: amount,
            lockup: Lockup {
                end_ts: 200,
                kind: None,
                period: LockupPeriod::None,
                ..Default::default()
            },
            is_used: true,
            ..Default::default()
        };
        assert_eq!(deposit.weighted_stake(50), 0);

        let deposit = DepositEntry {
            lockup: Lockup {
                kind: None,
                period: LockupPeriod::ThreeMonths,
                ..deposit.lockup
            },
            is_used: true,
            ..deposit
        };
        assert_eq!(deposit.weighted_stake(50), 0);

        let deposit = DepositEntry {
            lockup: Lockup {
                kind: Constant,
                period: LockupPeriod::None,
                ..deposit.lockup
            },
            is_used: true,
            ..deposit
        };
        assert_eq!(deposit.weighted_stake(50), 0);

        let deposit = DepositEntry {
            lockup: Lockup {
                kind: Constant,
                period: LockupPeriod::SixMonths,
                ..deposit.lockup
            },
            is_used: false,
            ..deposit
        };
        assert_eq!(deposit.weighted_stake(50), 0);
    }
}
