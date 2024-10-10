use crate::cpi_instructions;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount};
use bytemuck::bytes_of_mut;
use mplx_staking_states::{
    error::MplStakingError,
    state::{LockupKind, LockupPeriod, Registrar, Voter},
    voter_seeds,
};
use spl_associated_token_account::get_associated_token_address;
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
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
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
    #[account(
        mut,
        seeds = [b"mining", voter_authority.key().as_ref(), reward_pool.key().as_ref()],
        seeds::program = rewards_program.key(),
        bump,
    )]
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

/// Closes the voter account, and specified token vaults if provided in the remaining accounts,
/// allowing to retrieve rent examption SOL.
/// Only accounts with no remaining stakes can be closed.
///
/// Remaining accounts should containt the complete list of ATA that must be closed,
/// the length of those accounts should be equal to the number of mint configs in the registrar.
pub fn close_voter<'info>(ctx: Context<'_, '_, '_, 'info, CloseVoter<'info>>) -> Result<()> {
    let registrar = ctx.accounts.registrar.load()?;

    require!(
        ctx.accounts.rewards_program.key() == registrar.rewards_program,
        MplStakingError::InvalidRewardsProgram
    );

    require!(
        registrar.reward_pool == ctx.accounts.reward_pool.key(),
        MplStakingError::InvalidRewardPool
    );

    {
        let voter = ctx.accounts.voter.load()?;

        let any_locked = voter.deposits.iter().any(|d| d.amount_locked() > 0);
        require!(!any_locked, MplStakingError::DepositStillLocked);

        let active_deposit_entries = voter
            .deposits
            .iter()
            .filter(|d| {
                d.is_used
                    && d.lockup.kind != LockupKind::None
                    && d.lockup.period != LockupPeriod::None
            })
            .count();
        require_eq!(active_deposit_entries, 0, MplStakingError::DepositStillUsed);

        let voter_seeds = voter_seeds!(voter);

        // will close all the token accounts owned by the voter
        for deposit_vault_info in ctx.remaining_accounts {
            let deposit_vault_ta = Account::<TokenAccount>::try_from(deposit_vault_info)
                .map_err(|_| MplStakingError::DeserializationError)?;
            registrar.voting_mint_config_index(deposit_vault_ta.mint)?;

            require_keys_eq!(
                *deposit_vault_info.key,
                get_associated_token_address(&ctx.accounts.voter.key(), &deposit_vault_ta.mint),
            );

            require_keys_eq!(
                deposit_vault_ta.owner,
                ctx.accounts.voter.key(),
                MplStakingError::InvalidAuthority
            );
            require_eq!(
                deposit_vault_ta.amount,
                0,
                MplStakingError::VaultTokenNonZero
            );

            // close vault
            let cpi_close_accounts = CloseAccount {
                account: deposit_vault_ta.to_account_info(),
                destination: ctx.accounts.sol_destination.to_account_info(),
                authority: ctx.accounts.voter.to_account_info(),
            };
            token::close_account(CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_close_accounts,
                &[voter_seeds],
            ))?;

            deposit_vault_ta.exit(ctx.program_id)?;
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
