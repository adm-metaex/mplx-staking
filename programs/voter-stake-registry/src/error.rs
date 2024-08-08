use anchor_lang::prelude::*;

#[error_code]
pub enum MplStakingError {
    // 6000 / 0x1770
    #[msg("")]
    VotingMintNotFound,
    // 6001 / 0x1771
    #[msg("")]
    VotingTokenNonZero,
    // 6002 / 0x1772
    #[msg("")]
    OutOfBoundsDepositEntryIndex,
    // 6003 / 0x1773
    #[msg("")]
    UnusedDepositEntryIndex,
    // 6004 / 0x1774
    #[msg("")]
    InsufficientUnlockedTokens,
    // 6005 / 0x1775
    #[msg("")]
    InvalidLockupPeriod,
    // 6006 / 0x1776
    #[msg("")]
    VotingMintConfigIndexAlreadyInUse,
    // 6007 / 0x1777
    #[msg("")]
    OutOfBoundsVotingMintConfigIndex,
    // 6008 / 0x1778
    #[msg("")]
    ForbiddenCpi,
    // 6009 / 0x1779
    #[msg("")]
    InvalidMint,
    // 6010 / 0x177a
    #[msg("")]
    DepositStillLocked,
    // 6011 / 0x177b
    #[msg("")]
    InvalidAuthority,
    // 6012 / 0x177c
    #[msg("")]
    InvalidTokenOwnerRecord,
    // 6013 / 0x177d
    #[msg("")]
    InvalidRealmAuthority,
    // 6014 / 0x177e
    #[msg("")]
    VoterWeightOverflow,
    // 6015 / 0x177f
    #[msg("")]
    LockupSaturationMustBePositive,
    // 6016 / 0x1780
    #[msg("")]
    VotingMintConfiguredWithDifferentIndex,
    // 6017 / 0x1781
    #[msg("")]
    InternalProgramError,
    // 6018 / 0x1782
    #[msg("")]
    InvalidLockupKind,
    // 6019 / 0x1783
    #[msg("")]
    VaultTokenNonZero,
    // 6020 / 0x1784
    #[msg("")]
    InvalidTimestampArguments,
    // 6021 / 0x1785
    #[msg("")]
    UnlockMustBeCalledFirst,
    // 6022 / 0x1786
    #[msg("")]
    UnlockAlreadyRequested,
    // 6023 / 0x1787
    #[msg("")]
    ExtendDepositIsNotAllowed,
    // 6024 / 0x1788
    #[msg("To deposit additional tokens, extend the deposit")]
    DepositingIsForbidded,
    // 6025 / 0x1789
    #[msg("Cpi call must return data, but data is absent")]
    CpiReturnDataIsAbsent,
    // 6026 / 0x178a
    #[msg("The source for the transfer only can be a deposit on DAO")]
    LockingIsForbidded,
    // 6027 / 0x178b
    #[msg("Locking up tokens is only allowed for freshly-deposited deposit entry")]
    DepositEntryIsOld,
    // 6028 / 0x178c
    #[msg("Arithmetic operation has beed overflowed")]
    ArithmeticOverflow,
    // 6029 / 0x178d
    #[msg("Rewards: Delegate must have at least 15_000_000 of own weighted stake")]
    InsufficientWeightedStake,
    // 6030 / 0x178e
    #[msg("Rewards: Invalid delegate account")]
    InvalidDelegate,
    // 6031 / 0x178f
    #[msg("Rewards: Invalid mining account")]
    InvalidMining,
    // 6032 / 0x1790
    #[msg("Rewards: Updating delegate is sooner than 5 days")]
    DelegateUpdateIsTooSoon,
    // 6033 / 0x1791
    #[msg("Rewards: Cannot change delegate to the same delegate")]
    SameDelegate,
    // 6034 / 0x1792
    #[msg("Rewards: Invalid reward pool account")]
    InvalidRewardPool,
    // 6035 / 0x1793
    #[msg("Rewards: Passed remaining accounts are invalid, interaction with dao was'nt found")]
    NoDaoInteractionFound,
}
