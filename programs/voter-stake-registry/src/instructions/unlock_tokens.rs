use crate::state::LockupPeriod;
use crate::{clock_unix_timestamp, cpi_instructions::withdraw_mining, Stake};
use crate::{error::MplStakingError, state::COOLDOWN_SECS};
use anchor_lang::prelude::*;

pub fn unlock_tokens(ctx: Context<Stake>, deposit_entry_index: u8) -> Result<()> {
    let voter = &mut ctx.accounts.voter.load_mut()?;
    let curr_ts = clock_unix_timestamp();

    let deposit_entry = voter.active_deposit_mut(deposit_entry_index)?;

    // Check whether unlock request is allowed
    require!(
        !deposit_entry.lockup.cooldown_requested,
        MplStakingError::UnlockAlreadyRequested
    );
    require!(
        curr_ts >= deposit_entry.lockup.end_ts,
        MplStakingError::DepositStillLocked
    );

    ctx.accounts.verify_delegate_and_its_mining(deposit_entry)?;

    deposit_entry.lockup.cooldown_requested = true;
    if deposit_entry.lockup.period == LockupPeriod::Test {
        deposit_entry.lockup.cooldown_ends_at = curr_ts
            .checked_add(120)
            .ok_or(MplStakingError::InvalidTimestampArguments)?;
    } else {
        deposit_entry.lockup.cooldown_ends_at = curr_ts
            .checked_add(COOLDOWN_SECS)
            .ok_or(MplStakingError::InvalidTimestampArguments)?;
    }

    let rewards_program = ctx.accounts.rewards_program.to_account_info();
    let reward_pool = ctx.accounts.reward_pool.to_account_info();
    let mining = ctx.accounts.deposit_mining.to_account_info();
    let delegate_mining = ctx.accounts.delegate_mining.to_account_info();
    let owner = ctx.accounts.voter_authority.to_account_info();
    let registrar = ctx.accounts.registrar.load()?;
    let deposit_authority = ctx.accounts.registrar.to_account_info();
    let signers_seeds = &[
        registrar.realm.as_ref(),
        b"registrar".as_ref(),
        (registrar.realm_governing_token_mint.as_ref()),
        &[registrar.bump][..],
    ];

    withdraw_mining(
        rewards_program,
        reward_pool,
        mining,
        deposit_authority,
        delegate_mining,
        deposit_entry.amount_deposited_native,
        owner.key,
        signers_seeds,
    )?;

    Ok(())
}
