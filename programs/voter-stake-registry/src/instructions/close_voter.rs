use crate::cpi_instructions;
use crate::{
    error::MplStakingError,
    state::{Registrar, Voter},
    voter_seeds,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer};
use bytemuck::bytes_of_mut;
use std::ops::DerefMut;

// Remaining accounts must be all the token token accounts owned by voter, he wants to close,
// they should be writable so that they can be closed and sol required for rent
// can then be sent back to the sol_destination
#[derive(Accounts)]
pub struct CloseVoter<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [voter.load()?.registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.voter_bump,
        has_one = voter_authority,
        close = sol_destination
    )]
    pub voter: AccountLoader<'info, Voter>,

    // also, it's an owner of the mining_account
    pub voter_authority: Signer<'info>,

    /// CHECK: mining PDA will be checked in the rewards contract
    /// PDA(["mining", mining owner <aka voter_authority in our case>, reward_pool],
    /// reward_program)
    #[account(mut)]
    pub deposit_mining: UncheckedAccount<'info>,

    /// CHECK:
    /// Ownership of the account will be checked in the rewards contract
    /// It's the core account for the rewards contract, which will
    /// keep track of all rewards and staking logic.
    pub reward_pool: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Destination may be any address.
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    /// CHECK: Rewards Program account
    #[account(executable)]
    pub rewards_program: UncheckedAccount<'info>,
}

/// Closes the voter account, transfers all funds from token accounts and closes vaults.
/// Only accounts with no remaining lockups can be closed.
/// remaining_accounts: All voter vaults followed by target token accounts, in order.
pub fn close_voter<'info>(ctx: Context<'_, '_, '_, 'info, CloseVoter<'info>>) -> Result<()> {
    let registrar = ctx.accounts.registrar.load()?;

    require!(
        registrar.reward_pool == ctx.accounts.reward_pool.key(),
        MplStakingError::InvalidRewardPool
    );

    {
        let voter = ctx.accounts.voter.load()?;

        let any_locked = voter.deposits.iter().any(|d| d.amount_locked() > 0);
        require!(!any_locked, MplStakingError::DepositStillLocked);

        let active_deposit_entries = voter.deposits.iter().filter(|d| d.is_used).count();
        require_eq!(ctx.remaining_accounts.len(), active_deposit_entries);

        let voter_seeds = voter_seeds!(voter);

        let active_deposits = voter.deposits.iter().filter(|d| d.is_used);
        let deposit_vaults = &ctx.remaining_accounts[..active_deposit_entries];
        let target_accounts = &ctx.remaining_accounts[active_deposit_entries..];

        for ((deposit, deposit_vault), target_account) in
            active_deposits.zip(deposit_vaults).zip(target_accounts)
        {
            let mint = &registrar.voting_mints[deposit.voting_mint_config_idx as usize].mint;

            let token = Account::<TokenAccount>::try_from(&deposit_vault.clone()).unwrap();
            require_keys_eq!(
                token.owner,
                ctx.accounts.voter.key(),
                MplStakingError::InvalidAuthority
            );
            require_keys_eq!(token.mint, *mint);
            require_eq!(token.amount, 0, MplStakingError::VaultTokenNonZero);

            // transfer to target_account
            let cpi_transfer_accounts = Transfer {
                from: deposit_vault.to_account_info(),
                to: target_account.to_account_info(),
                authority: ctx.accounts.voter.to_account_info(),
            };
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    cpi_transfer_accounts,
                    &[voter_seeds],
                ),
                token.amount,
            )?;

            // close vault
            let cpi_close_accounts = CloseAccount {
                account: deposit_vault.to_account_info(),
                destination: ctx.accounts.sol_destination.to_account_info(),
                authority: ctx.accounts.voter.to_account_info(),
            };
            token::close_account(CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_close_accounts,
                &[voter_seeds],
            ))?;

            deposit_vault.exit(ctx.program_id)?;
        }
    }

    {
        // zero out voter account to prevent reinit attacks
        let mut voter = ctx.accounts.voter.load_mut()?;
        let voter_dereffed = voter.deref_mut();
        let voter_bytes = bytes_of_mut(voter_dereffed);
        voter_bytes.fill(0);
    }

    let reward_pool = &ctx.accounts.reward_pool;
    let mining = &ctx.accounts.deposit_mining;
    let mining_owner = &ctx.accounts.voter_authority;
    let deposit_authority = &ctx.accounts.registrar.to_account_info();
    let target_account = &ctx.accounts.sol_destination.to_account_info();
    let signers_seeds = &[
        &registrar.realm.key().to_bytes(),
        b"registrar".as_ref(),
        &registrar.realm_governing_token_mint.key().to_bytes(),
        &[registrar.bump][..],
    ];

    cpi_instructions::close_mining(
        ctx.accounts.rewards_program.to_account_info(),
        mining.to_account_info(),
        mining_owner.to_account_info(),
        target_account.to_account_info(),
        deposit_authority.to_account_info(),
        reward_pool.to_account_info(),
        signers_seeds,
    )?;

    Ok(())
}
