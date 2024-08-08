use crate::events::*;
use crate::state::{Registrar, Voter};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct LogVoterInfo<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(has_one = registrar)]
    pub voter: AccountLoader<'info, Voter>,
}

/// A no-effect instruction that logs information about the voter and deposits.
///
/// Logs deposit information about deposits with an index between `deposit_entry_begin`
/// and `deposit_entry_begin + deposit_entry_count`.
///
/// With the current setup, all information about deposits can be logged by calling
/// this with deposit_entry_begin=0, =8, =16, =24 and deposit_entry_count=8.
pub fn log_voter_info(
    ctx: Context<LogVoterInfo>,
    deposit_entry_begin: u8,
    deposit_entry_count: u8,
) -> Result<()> {
    let voter = ctx.accounts.voter.load()?;
    let deposit_entry_begin = deposit_entry_begin as usize;
    let deposit_entry_count = deposit_entry_count as usize;

    msg!("voter");
    emit!(VoterInfo {
        voting_power: voter.weight()?,
        voting_power_baseline: voter.weight_baseline(),
    });

    msg!("deposit_entries");
    for (deposit_index, deposit) in voter.deposits.iter().enumerate() {
        if !deposit.is_used
            || deposit_index < deposit_entry_begin
            || deposit_index >= deposit_entry_begin + deposit_entry_count
        {
            continue;
        }
        let locking_info = if !deposit.lockup.cooldown_requested {
            Some(LockingInfo {
                amount: deposit.amount_locked(),
                end_timestamp: Some(deposit.lockup.end_ts),
                vesting: None,
            })
        } else {
            None
        };

        emit!(DepositEntryInfo {
            deposit_entry_index: deposit_index as u8,
            voting_mint_config_index: deposit.voting_mint_config_idx,
            unlocked: deposit.amount_unlocked(),
            voting_power: deposit.voting_power()?,
            voting_power_baseline: deposit.amount_deposited_native,
            locking: locking_info,
        });
    }
    Ok(())
}
