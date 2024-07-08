use crate::{clock_unix_timestamp, cpi_instructions::withdraw_mining};
use anchor_lang::prelude::*;
use mplx_staking_states::{
    error::VsrError,
    state::{Registrar, Voter, COOLDOWN_SECS},
};

#[derive(Accounts)]
pub struct UnlockTokens<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.voter_bump,
        has_one = voter_authority,
        has_one = registrar)]
    pub voter: AccountLoader<'info, Voter>,
    pub voter_authority: Signer<'info>,

    /// CHECK: Reward Pool PDA will be checked in the rewards contract
    #[account(mut)]
    pub reward_pool: UncheckedAccount<'info>,

    /// CHECK: mining PDA will be checked in the rewards contract
    #[account(mut)]
    pub deposit_mining: UncheckedAccount<'info>,

    /// CHECK: Rewards Program account
    #[account(executable)]
    pub rewards_program: UncheckedAccount<'info>,
}

pub fn unlock_tokens(ctx: Context<UnlockTokens>, deposit_entry_index: u8) -> Result<()> {
    let voter = &mut ctx.accounts.voter.load_mut()?;
    let curr_ts = clock_unix_timestamp();

    let deposit_entry = voter.active_deposit_mut(deposit_entry_index)?;

    // Check whether unlock request is allowed
    require!(
        !deposit_entry.lockup.cooldown_requested,
        VsrError::UnlockAlreadyRequested
    );
    require!(
        curr_ts >= deposit_entry.lockup.end_ts,
        VsrError::DepositStillLocked
    );

    deposit_entry.lockup.cooldown_requested = true;
    deposit_entry.lockup.cooldown_ends_at = curr_ts
        .checked_add(COOLDOWN_SECS)
        .ok_or(VsrError::InvalidTimestampArguments)?;

    let rewards_program = &ctx.accounts.rewards_program;
    let reward_pool = &ctx.accounts.reward_pool;
    let mining = &ctx.accounts.deposit_mining;

    let owner = &ctx.accounts.voter_authority;
    let registrar = &ctx.accounts.registrar.load()?;
    let signers_seeds = &[
        registrar.realm.as_ref(),
        b"registrar".as_ref(),
        (registrar.realm_governing_token_mint.as_ref()),
        &[registrar.bump][..],
    ];
    let pool_deposit_authority = &ctx.accounts.registrar;

    withdraw_mining(
        rewards_program.to_account_info(),
        reward_pool.to_account_info(),
        mining.to_account_info(),
        pool_deposit_authority.to_account_info(),
        deposit_entry.amount_deposited_native,
        owner.key,
        signers_seeds,
    )?;

    Ok(())
}
