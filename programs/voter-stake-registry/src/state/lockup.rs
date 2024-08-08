use crate::borsh::{BorshDeserialize, BorshSerialize};
use crate::error::*;
use anchor_lang::prelude::*;
use static_assertions::const_assert;

/// Seconds in one day.
pub const SECS_PER_DAY: u64 = 86_400;

/// Seconds in one month.
pub const SECS_PER_MONTH: u64 = 365 * SECS_PER_DAY / 12;

/// Seconds in cooldown (5 days)
pub const COOLDOWN_SECS: u64 = 86_400 * 5;

#[zero_copy]
#[derive(Default, Debug)]
pub struct Lockup {
    /// Start of the lockup.
    pub start_ts: u64,

    /// End of the lockup.
    pub end_ts: u64,

    /// End of the cooldown.
    pub cooldown_ends_at: u64,

    pub cooldown_requested: bool,
    /// Type of lockup.
    pub kind: LockupKind,

    /// Type of lockup
    pub period: LockupPeriod,
    /// Reserved for future use
    pub _reserved0: [u8; 16],
    /// Padding after period to align the struct size to 8 bytes
    pub _reserved1: [u8; 5],
}
const_assert!(std::mem::size_of::<Lockup>() == 3 * 8 + 1 + 1 + 1 + 16 + 5);
const_assert!(std::mem::size_of::<Lockup>() % 8 == 0);

impl Lockup {
    /// Create lockup for a given period
    pub fn new(kind: LockupKind, start_ts: u64, period: LockupPeriod) -> Result<Self> {
        require!(
            (kind == LockupKind::None && period == LockupPeriod::None)
                || (kind == LockupKind::Constant && period != LockupPeriod::None),
            MplStakingError::InvalidLockupKind
        );

        let end_ts = start_ts
            .checked_add(period.to_secs())
            .ok_or(MplStakingError::InvalidTimestampArguments)?;

        Ok(Self {
            kind,
            start_ts,
            end_ts,
            period,
            // 0 means cooldown hasn't been requested
            cooldown_ends_at: 0,
            cooldown_requested: false,
            _reserved0: [0; 16],
            _reserved1: [0; 5],
        })
    }

    /// True when the lockup is finished.
    pub fn expired(&self, curr_ts: u64) -> bool {
        self.seconds_left(curr_ts) == 0
    }

    /// Number of seconds left in the lockup.
    /// May be more than end_ts-start_ts if curr_ts < start_ts.
    pub fn seconds_left(&self, curr_ts: u64) -> u64 {
        // if self.kind == LockupKind::Constant{
        //     curr_ts = self.start_ts;
        // };
        if self.kind == LockupKind::None {
            return 0;
        }

        if curr_ts >= self.end_ts {
            0
        } else {
            self.end_ts - curr_ts
        }
    }

    /// Returns the number of periods left on the lockup.
    /// Returns 0 after lockup has expired and periods_total before start_ts.
    pub fn periods_left(&self, curr_ts: u64) -> Result<u64> {
        let period_secs = self.kind.period_secs();
        if period_secs == 0 {
            return Ok(0);
        }
        if curr_ts < self.start_ts {
            return self.periods_total();
        }
        Ok(self
            .seconds_left(curr_ts)
            .checked_add(period_secs.saturating_sub(1))
            .unwrap()
            .checked_div(period_secs)
            .unwrap())
    }

    /// Returns the current period in the vesting schedule.
    /// Will report periods_total() after lockup has expired and 0 before start_ts.
    pub fn period_current(&self, curr_ts: u64) -> Result<u64> {
        Ok(self
            .periods_total()?
            .saturating_sub(self.periods_left(curr_ts)?))
    }

    /// Returns the total amount of periods in the lockup.
    pub fn periods_total(&self) -> Result<u64> {
        let period_secs = self.kind.period_secs();
        if period_secs == 0 {
            return Ok(0);
        }

        let lockup_secs = self.seconds_left(self.start_ts);
        require_eq!(
            lockup_secs % period_secs,
            0,
            MplStakingError::InvalidLockupPeriod
        );

        Ok(lockup_secs.checked_div(period_secs).unwrap())
    }

    /// Remove the vesting periods that are now in the past.
    pub fn remove_past_periods(&mut self, curr_ts: u64) -> Result<()> {
        let periods = self.period_current(curr_ts)?;
        let period_secs = self.kind.period_secs();
        self.start_ts = self
            .start_ts
            .checked_add(
                periods
                    .checked_mul(period_secs)
                    .ok_or(MplStakingError::InvalidTimestampArguments)?,
            )
            .unwrap();
        require_gte!(
            self.end_ts,
            self.start_ts,
            MplStakingError::InternalProgramError
        );
        require_eq!(
            self.period_current(curr_ts)?,
            0,
            MplStakingError::InternalProgramError
        );
        Ok(())
    }

    /// Multiplier of the lockup period at the specified timestamp.
    /// If the lockup has expired, the multiplier will be as for the Flex period.
    /// If the lockup has not started yet, the multiplier will be zero.
    /// If the cooldown has been requested, the multiplier will be zero.
    /// Otherwise, the multiplier will be the one for the lockup period.
    /// The multiplier is used to calculate the weighted stake.
    #[inline(always)]
    pub fn multiplier(&self, curr_ts: u64) -> u64 {
        if curr_ts < self.start_ts || self.cooldown_requested {
            return 0;
        }
        if self.end_ts > curr_ts {
            self.period.multiplier()
        } else {
            LockupPeriod::Flex.multiplier()
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockupPeriod {
    None,
    Test,
    Flex,
    ThreeMonths,
    SixMonths,
    OneYear,
}

impl Default for LockupPeriod {
    fn default() -> Self {
        Self::Flex
    }
}

impl LockupPeriod {
    pub fn to_secs(&self) -> u64 {
        match self {
            LockupPeriod::ThreeMonths => SECS_PER_MONTH * 3,
            LockupPeriod::SixMonths => SECS_PER_MONTH * 6,
            LockupPeriod::OneYear => SECS_PER_MONTH * 12,
            LockupPeriod::Flex => SECS_PER_DAY * 5,
            LockupPeriod::None => 0,
            LockupPeriod::Test => 120,
        }
    }

    pub fn multiplier(&self) -> u64 {
        match self {
            LockupPeriod::None => 0,
            LockupPeriod::ThreeMonths => 2,
            LockupPeriod::SixMonths => 4,
            LockupPeriod::OneYear => 6,
            LockupPeriod::Flex => 1,
            LockupPeriod::Test => 1,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub enum LockupKind {
    /// No lockup, tokens can be withdrawn as long as not engaged in a proposal.
    None,
    /// Lock up permanently. The number of days specified becomes the minimum
    /// unlock period when the deposit (or a part of it) is changed to None.
    Constant,
}

impl Default for LockupKind {
    fn default() -> Self {
        Self::Constant
    }
}

impl LockupKind {
    /// The lockup length is specified by passing the number of lockup periods
    /// to create_deposit_entry. This describes a period's length.
    ///
    /// For vesting lockups, the period length is also the vesting period.
    pub fn period_secs(&self) -> u64 {
        match self {
            LockupKind::None => 0,
            LockupKind::Constant => SECS_PER_DAY, // arbitrary choice
        }
    }

    /// Lockups cannot decrease in strictness
    pub fn strictness(&self) -> u8 {
        match self {
            LockupKind::None => 0,
            LockupKind::Constant => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockup_multiplier_not_started() {
        let lockup = Lockup::new(LockupKind::Constant, 100, LockupPeriod::ThreeMonths).unwrap();
        assert_eq!(lockup.multiplier(99), 0);
    }

    #[test]
    fn test_lockup_multiplier_started() {
        let lockup = Lockup::new(LockupKind::Constant, 100, LockupPeriod::ThreeMonths).unwrap();
        assert_eq!(lockup.multiplier(100), 2);
    }

    #[test]
    fn test_lockup_multiplier_expired() {
        let period = LockupPeriod::ThreeMonths;
        let lockup = Lockup::new(LockupKind::Constant, 100, period).unwrap();
        assert_eq!(lockup.multiplier(200 + period.to_secs()), 1);
    }

    #[test]
    fn test_lockup_multiplier_cooldown() {
        let lockup = Lockup::new(LockupKind::Constant, 100, LockupPeriod::ThreeMonths).unwrap();
        let mut lockup = lockup;
        lockup.cooldown_requested = true;
        assert_eq!(lockup.multiplier(200), 0);
    }
}
