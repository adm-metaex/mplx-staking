use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use mplx_staking_states::{
    error::MplStakingError,
    state::{Registrar, VotingMintConfig},
};

// Remaining accounts must be all the token mints that have registered
// as voting mints, including the newly registered one.
#[derive(Accounts)]
pub struct ConfigureVotingMint<'info> {
    #[account(mut, has_one = realm_authority)]
    pub registrar: AccountLoader<'info, Registrar>,
    pub realm_authority: Signer<'info>,

    /// Tokens of this mint will produce vote weight
    pub mint: Account<'info, Mint>,
    // This instruction expects that all voting mint addresses, including a
    // newly registered one, are passed in ctx.remainingAccounts.
}

/// Creates a new exchange rate for a given mint. This allows a voter to
/// deposit the mint in exchange for vote weight. There can only be a single
/// exchange rate per mint.
///
/// * `idx`: index of the rate to be set
/// * `grant_authority`: The keypair that might be an authority for Grant/Clawback
///
/// This instruction can be called several times for the same mint and index to
/// change the voting mint configuration.
///
/// The vote weight for `amount` of native tokens will be 1:1. Therefore, all active
/// deposited tokens (locked or not) will be sumed up.

pub fn configure_voting_mint(
    ctx: Context<ConfigureVotingMint>,
    idx: u16,
    grant_authority: Option<Pubkey>,
) -> Result<()> {
    let registrar = &mut ctx.accounts.registrar.load_mut()?;
    let mint = ctx.accounts.mint.key();
    let idx = idx as usize;
    require_gt!(
        registrar.voting_mints.len(),
        idx,
        MplStakingError::OutOfBoundsVotingMintConfigIndex
    );

    // Either it's reconfiguring an existing mint with the correct index,
    // or configuring a new mint on an unused index.
    match registrar.voting_mint_config_index(mint) {
        Ok(existing_idx) => require_eq!(
            existing_idx,
            idx,
            MplStakingError::VotingMintConfiguredWithDifferentIndex
        ),
        Err(_) => require!(
            !registrar.voting_mints[idx].in_use(),
            MplStakingError::VotingMintConfigIndexAlreadyInUse
        ),
    };

    registrar.voting_mints[idx] = VotingMintConfig {
        mint,
        grant_authority: grant_authority.unwrap_or_default(),
    };

    // Check for overflow in vote weight
    registrar.max_vote_weight(ctx.remaining_accounts)?;

    Ok(())
}
