use crate::{clock_unix_timestamp, cpi_instructions, Stake};
use anchor_lang::prelude::*;
use mplx_staking_states::{
    error::VsrError,
    state::{LockupKind, LockupPeriod},
};

/// Prolongs the deposit
///
/// The stake will be extended with the same lockup period as it was previously in case it's not
/// expired. If the deposit has expired, it can be extended with any LockupPeriod.
/// The deposit entry must have been initialized with create_deposit_entry.
///
/// `deposit_entry_index`: Index of the deposit entry.
pub fn extend_stake(
    ctx: Context<Stake>,
    source_deposit_entry_index: u8,
    target_deposit_entry_index: u8,
    new_lockup_period: LockupPeriod,
    additional_amount: u64,
) -> Result<()> {
    let registrar = &ctx.accounts.registrar.load()?;
    let curr_ts = clock_unix_timestamp();

    let voter = &mut ctx.accounts.voter.load_mut()?;

    let source = voter.active_deposit_mut(source_deposit_entry_index)?;
    let source_mint_idx = source.voting_mint_config_idx;
    let source_available_tokens = source.amount_unlocked(curr_ts);
    require!(
        source.lockup.kind == LockupKind::None,
        VsrError::LockingIsForbidded
    );
    source.amount_deposited_native = source
        .amount_deposited_native
        .checked_sub(additional_amount)
        .ok_or(VsrError::ArithmeticOverflow)?;

    let target = voter.active_deposit_mut(target_deposit_entry_index)?;
    require_gte!(
        source_available_tokens,
        additional_amount,
        VsrError::InsufficientUnlockedTokens
    );
    require!(
        target.lockup.period != LockupPeriod::None && target.lockup.kind != LockupKind::None,
        VsrError::ExtendDepositIsNotAllowed
    );

    let start_ts = target.lockup.start_ts;
    let target_basic_amount = target.amount_deposited_native;
    let current_lockup_period = if target.lockup.expired(curr_ts) {
        LockupPeriod::Flex
    } else {
        target.lockup.period
    };

    // different type of deposit is only allowed if
    // the current deposit has expired
    require!(
        new_lockup_period >= current_lockup_period,
        VsrError::ExtendDepositIsNotAllowed
    );

    // Check target compatibility
    require_eq!(
        target.voting_mint_config_idx,
        source_mint_idx,
        VsrError::InvalidMint
    );
    target.amount_deposited_native = target
        .amount_deposited_native
        .checked_add(additional_amount)
        .ok_or(VsrError::ArithmeticOverflow)?;
    target.lockup.start_ts = curr_ts;
    target.lockup.end_ts = curr_ts
        .checked_add(new_lockup_period.to_secs())
        .ok_or(VsrError::InvalidTimestampArguments)?;
    target.lockup.period = new_lockup_period;
    target.delegate_mining = ctx.accounts.delegate_mining.key();

    let reward_pool = ctx.accounts.reward_pool.to_account_info();
    let mining = ctx.accounts.deposit_mining.to_account_info();
    let deposit_authority = ctx.accounts.registrar.to_account_info();
    let delegate_mining = ctx.accounts.delegate_mining.to_account_info();
    let signers_seeds = &[
        &registrar.realm.key().to_bytes(),
        b"registrar".as_ref(),
        &registrar.realm_governing_token_mint.key().to_bytes(),
        &[registrar.bump][..],
    ];
    let mining_owner = &ctx.accounts.voter_authority.key();

    cpi_instructions::extend_stake(
        ctx.accounts.rewards_program.to_account_info(),
        reward_pool,
        mining.to_account_info(),
        deposit_authority,
        delegate_mining,
        current_lockup_period,
        new_lockup_period,
        start_ts,
        target_basic_amount,
        additional_amount,
        mining_owner,
        signers_seeds,
    )?;

    Ok(())
}
