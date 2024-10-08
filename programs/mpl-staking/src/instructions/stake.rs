use crate::{clock_unix_timestamp, cpi_instructions, Stake};
use anchor_lang::prelude::*;
use mplx_staking_states::{error::MplStakingError, registrar_seeds, state::LockupKind};

/// Transfers unlocked tokens from the source deposit entry to the target deposit entry.
///
/// Transfers token from one DepositEntry that is not LockupKind::None to another that is
/// LockupKind::Constant. In terms of business logic that means we want to deposit some tokens on
/// DAO, then we want to lock them up in order to receice rewards
pub fn stake(
    ctx: Context<Stake>,
    source_deposit_entry_index: u8,
    target_deposit_entry_index: u8,
    amount: u64,
) -> Result<()> {
    let registrar = ctx.accounts.registrar.load()?;

    require!(
        registrar.reward_pool == ctx.accounts.reward_pool.key(),
        MplStakingError::InvalidRewardPool
    );

    let voter = &mut ctx.accounts.voter.load_mut()?;

    let source = voter.active_deposit_mut(source_deposit_entry_index)?;
    let source_mint_idx = source.voting_mint_config_idx;
    require!(
        source.lockup.kind == LockupKind::None,
        MplStakingError::LockingIsForbidded
    );

    // Reduce source amounts
    require_gte!(
        source.amount_unlocked(),
        amount,
        MplStakingError::InsufficientUnlockedTokens
    );
    source.amount_deposited_native = source
        .amount_deposited_native
        .checked_sub(amount)
        .ok_or(MplStakingError::ArithmeticOverflow)?;

    // Check target compatibility
    let target = voter.active_deposit_mut(target_deposit_entry_index)?;
    require_eq!(
        target.voting_mint_config_idx,
        source_mint_idx,
        MplStakingError::InvalidMint
    );

    // Checks that target doesn't have any stored tokens yet
    require!(
        target.amount_deposited_native == 0,
        MplStakingError::DepositEntryIsOld
    );
    ctx.accounts.verify_delegate(target)?;

    // Add target amounts
    target.amount_deposited_native = target
        .amount_deposited_native
        .checked_add(amount)
        .ok_or(MplStakingError::ArithmeticOverflow)?;
    target.delegate_last_update_ts = clock_unix_timestamp();
    target.lockup.end_ts = target
        .lockup
        .start_ts
        .checked_add(target.lockup.period.to_secs())
        .ok_or(MplStakingError::InvalidTimestampArguments)?;

    let reward_pool = ctx.accounts.reward_pool.to_account_info();
    let mining = ctx.accounts.deposit_mining.to_account_info();
    let deposit_authority = ctx.accounts.registrar.to_account_info();
    let delegate_mining = ctx.accounts.delegate_mining.to_account_info();
    let signers_seeds = registrar_seeds!(&registrar);
    let owner = &ctx.accounts.voter_authority.key();
    let delegate_wallet_addr = &ctx.accounts.delegate.key();

    cpi_instructions::deposit_mining(
        ctx.accounts.rewards_program.to_account_info(),
        reward_pool,
        mining,
        deposit_authority,
        delegate_mining,
        amount,
        target.lockup.period,
        owner,
        signers_seeds,
        delegate_wallet_addr,
    )?;

    Ok(())
}
