use crate::clock_unix_timestamp;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mplx_staking_states::{
    error::MplStakingError,
    state::{DepositEntry, Lockup, LockupKind, LockupPeriod, Registrar, Voter},
};

#[derive(Accounts)]
pub struct CreateDepositEntry<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.voter_bump,
        has_one = registrar,
        has_one = voter_authority)]
    pub voter: AccountLoader<'info, Voter>,

    #[account(
        init_if_needed,
        associated_token::authority = voter,
        associated_token::mint = deposit_mint,
        payer = payer
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub voter_authority: Signer<'info>,
    pub delegate_voter: AccountLoader<'info, Voter>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub deposit_mint: Box<Account<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

/// Creates a new deposit entry.
///
/// Initializes a deposit entry with the requested settings.
/// Will error if the deposit entry is already in use.
///
/// - `deposit_entry_index`: deposit entry to use
/// - `kind`: Type of lockup to use.
/// - `period`: An enum that represents possible options for locking up.
pub fn create_deposit_entry(
    ctx: Context<CreateDepositEntry>,
    deposit_entry_index: u8,
    kind: LockupKind,
    period: LockupPeriod,
) -> Result<()> {
    // Load accounts.
    let registrar = &ctx.accounts.registrar.load()?;
    let mut voter = ctx.accounts.voter.load_mut()?;

    let delegate = if ctx.accounts.delegate_voter.key() != ctx.accounts.voter.key() {
        let curr_ts = clock_unix_timestamp();
        let delegate_voter = ctx.accounts.delegate_voter.load()?;

        let delegate_voter_weighted_stake = delegate_voter
            .deposits
            .iter()
            .fold(0, |acc, d| acc + d.weighted_stake(curr_ts));
        require!(
            delegate_voter_weighted_stake >= Voter::MIN_OWN_WEIGHTED_STAKE,
            MplStakingError::InsufficientUnlockedTokens
        );

        delegate_voter.voter_authority.key()
    } else {
        voter.voter_authority.key()
    };

    // if both period and lockup are None, that means the deposit entry is not lockable
    // in that case delegate field doesn't make sense and should be the same as mining account
    // derived from voter
    if period == LockupPeriod::None && kind == LockupKind::None {
        require!(
            delegate == voter.voter_authority.key(),
            MplStakingError::InvalidDelegate
        );
    }

    // Get the exchange rate entry associated with this deposit.
    let mint_idx = registrar.voting_mint_config_index(ctx.accounts.deposit_mint.key())?;

    // Get and set up the deposit entry.
    require_gt!(
        voter.deposits.len(),
        deposit_entry_index as usize,
        MplStakingError::OutOfBoundsDepositEntryIndex
    );
    let d_entry = &mut voter.deposits[deposit_entry_index as usize];
    require!(!d_entry.is_used, MplStakingError::UnusedDepositEntryIndex);

    let start_ts = clock_unix_timestamp();

    *d_entry = DepositEntry {
        delegate,
        is_used: true,
        voting_mint_config_idx: mint_idx as u8,
        lockup: Lockup::new(kind, start_ts, period)?,
        ..Default::default()
    };

    Ok(())
}
