use crate::{
    clock_unix_timestamp,
    voter::{load_token_owner_record, VoterWeightRecord},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use mpl_common_constants::constants::GOVERNANCE_PROGRAM_ID;
use mplx_staking_states::{
    error::MplStakingError,
    state::{DepositEntry, LockupKind, LockupPeriod, Registrar, Voter},
    voter_seeds,
};
use spl_governance::state::realm::get_governing_token_holding_address_seeds;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.voter_bump,
        has_one = registrar,
        has_one = voter_authority,
    )]
    pub voter: AccountLoader<'info, Voter>,
    pub voter_authority: Signer<'info>,

    /// The token_owner_record for the voter_authority. This is needed
    /// to be able to forbid withdraws while the voter is engaged with
    /// a vote or has an open proposal.
    ///
    /// CHECK: token_owner_record is validated in the instruction:
    /// - owned by registrar.governance_program_id
    /// - for the registrar.realm
    /// - for the registrar.realm_governing_token_mint
    /// - governing_token_owner is voter_authority
    pub token_owner_record: UncheckedAccount<'info>,

    /// Withdraws must update the voter weight record, to prevent a stale
    /// record being used to vote after the withdraw.
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter-weight-record".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.voter_weight_record_bump,
        constraint = voter_weight_record.realm == registrar.load()?.realm,
        constraint = voter_weight_record.governing_token_owner == voter.load()?.voter_authority,
        constraint = voter_weight_record.governing_token_mint == registrar.load()?.realm_governing_token_mint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    #[account(
        mut,
        associated_token::authority = voter,
        associated_token::mint = destination.mint,
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub destination: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub realm_treasury: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_ctx(
        &self,
        destination: AccountInfo<'info>,
    ) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.vault.to_account_info(),
            to: destination,
            authority: self.voter.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

/// Withdraws tokens from a deposit entry, if they are unlocked
///
/// `deposit_entry_index`: The deposit entry to withdraw from.
/// `amount` is in units of the native currency being withdrawn.
pub fn withdraw(ctx: Context<Withdraw>, deposit_entry_index: u8, amount: u64) -> Result<()> {
    let slashing_penalty = {
        // Load the accounts.
        let registrar = &ctx.accounts.registrar.load()?;
        let voter = &mut ctx.accounts.voter.load_mut()?;

        require!(
            !voter.is_tokenflow_restricted(),
            MplStakingError::TokenflowRestricted
        );

        // Get the exchange rate for the token being withdrawn.
        let mint_idx = registrar.voting_mint_config_index(ctx.accounts.destination.mint)?;

        // Governance may forbid withdraws, for example when engaged in a vote.
        // Not applicable for tokens that don't contribute to voting power.
        let token_owner_record = load_token_owner_record(
            &voter.voter_authority,
            &ctx.accounts.token_owner_record.to_account_info(),
            registrar,
        )?;
        token_owner_record.assert_can_withdraw_governing_tokens()?;

        // Get the deposit being withdrawn from.
        let curr_ts = clock_unix_timestamp();
        let deposit_entry = voter.active_deposit_mut(deposit_entry_index)?;
        let slashing_penalty = deposit_entry.slashing_penalty;

        // check whether funds are cooled down
        if deposit_entry.lockup.kind == LockupKind::Constant {
            require!(
                deposit_entry.lockup.cooldown_requested,
                MplStakingError::UnlockMustBeCalledFirst
            );
            require!(
                curr_ts >= deposit_entry.lockup.cooldown_ends_at,
                MplStakingError::InvalidTimestampArguments
            );
        }

        require_gte!(
            deposit_entry.amount_unlocked(),
            amount,
            MplStakingError::InsufficientUnlockedTokens
        );
        require_eq!(
            mint_idx,
            deposit_entry.voting_mint_config_idx as usize,
            MplStakingError::InvalidMint
        );

        // Bookkeeping for withdrawn funds.
        require_gte!(
            deposit_entry.amount_deposited_native,
            amount,
            MplStakingError::InternalProgramError
        );

        deposit_entry.amount_deposited_native = deposit_entry
            .amount_deposited_native
            .checked_sub(amount)
            .ok_or(MplStakingError::ArithmeticOverflow)?;
        deposit_entry.slashing_penalty = deposit_entry
            .slashing_penalty
            .checked_sub(slashing_penalty)
            .ok_or(MplStakingError::ArithmeticOverflow)?;

        msg!(
            "Withdrew amount {} at deposit index {} with lockup kind {:?} and {} seconds left",
            amount,
            deposit_entry_index,
            deposit_entry.lockup.kind,
            deposit_entry.lockup.seconds_left(curr_ts),
        );

        if deposit_entry.amount_deposited_native == 0
            && deposit_entry.lockup.kind != LockupKind::None
            && deposit_entry.lockup.period != LockupPeriod::None
            && deposit_entry.slashing_penalty == 0
        {
            *deposit_entry = DepositEntry::default();
            deposit_entry.is_used = false;
        }

        // Update the voter weight record
        let record = &mut ctx.accounts.voter_weight_record;
        record.voter_weight = voter.weight()?;
        record.voter_weight_expiry = Some(Clock::get()?.slot);

        slashing_penalty
    };

    // Transfer the tokens
    {
        let voter = ctx.accounts.voter.load()?;
        let destination = ctx.accounts.destination.to_account_info();
        let voter_seeds = voter_seeds!(voter);
        token::transfer(
            ctx.accounts
                .transfer_ctx(destination)
                .with_signer(&[voter_seeds]),
            amount,
        )?;

        if slashing_penalty > 0 {
            let registrar = ctx.accounts.registrar.load()?;
            let realm_treasury = ctx.accounts.realm_treasury.to_account_info();
            let treasury_seeds = get_governing_token_holding_address_seeds(
                &registrar.realm,
                &registrar.realm_governing_token_mint,
            );
            let (treasury_addr, _) =
                Pubkey::find_program_address(&treasury_seeds, &Pubkey::from(GOVERNANCE_PROGRAM_ID));

            require!(
                treasury_addr == realm_treasury.key(),
                MplStakingError::InvalidTreasury
            );

            token::transfer(
                ctx.accounts
                    .transfer_ctx(realm_treasury)
                    .with_signer(&[voter_seeds]),
                slashing_penalty,
            )?;
        }
    }

    Ok(())
}
