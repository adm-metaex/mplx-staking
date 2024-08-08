use anchor_lang::prelude::*;
use instructions::*;

pub use error::MplStakingError;
pub use state::{
    lockup::{LockupKind, LockupPeriod},
    DepositEntry, Registrar, Voter,
};

pub mod cpi_instructions;
pub mod error;
pub mod events;
mod governance;
mod instructions;
pub mod state;
pub mod voter;

// The program address.
declare_id!("9XZ7Ku7FYGVk3veKba6BRKTFXoYJyh4b4ZHC6MfaTUE8");

/// # Introduction
///
/// The governance registry is an "addin" to the SPL governance program that
/// allows one to both vote with many different ypes of tokens for voting and to
/// scale voting power as a linear function of time locked--subject to some
/// maximum upper bound.
///
/// The flow for voting with this program is as follows:
///
/// - Create a SPL governance realm.
/// - Create a governance registry account.
/// - Add exchange rates for any tokens one wants to deposit. For example, if one wants to vote with
///   tokens A and B, where token B has twice the voting power of token A, then the exchange rate of
///   B would be 2 and the exchange rate of A would be 1.
/// - Create a voter account.
/// - Deposit tokens into this program, with an optional lockup period.
/// - Vote.
///
/// Upon voting with SPL governance, a client is expected to call
/// `decay_voting_power` to get an up to date measurement of a given `Voter`'s
/// voting power for the given slot. If this is not done, then the transaction
/// will fail (since the SPL governance program will require the measurement
/// to be active for the current slot).
///
/// # Interacting with SPL Governance
///
/// This program does not directly interact with SPL governance via CPI.
/// Instead, it simply writes a `VoterWeightRecord` account with a well defined
/// format, which is then used by SPL governance as the voting power measurement
/// for a given user.
///
/// # Max Vote Weight
///
/// Given that one can use multiple tokens to vote, the max vote weight needs
/// to be a function of the total supply of all tokens, converted into a common
/// currency. For example, if you have Token A and Token B, where 1 Token B =
/// 10 Token A, then the `max_vote_weight` should be `supply(A) + supply(B)*10`
/// where both are converted into common decimals. Then, when calculating the
/// weight of an individual voter, one can convert B into A via the given
/// exchange rate, which must be fixed.
///
/// Note that the above also implies that the `max_vote_weight` must fit into
/// a u64.
#[program]
pub mod mpl_staking {
    use super::*;

    pub fn create_registrar(
        ctx: Context<CreateRegistrar>,
        registrar_bump: u8,
        fill_authority: Pubkey,
        distribution_authority: Pubkey,
    ) -> Result<()> {
        instructions::create_registrar(ctx, registrar_bump, fill_authority, distribution_authority)
    }

    pub fn configure_voting_mint(
        ctx: Context<ConfigureVotingMint>,
        idx: u16,
        grant_authority: Option<Pubkey>,
    ) -> Result<()> {
        instructions::configure_voting_mint(ctx, idx, grant_authority)
    }

    pub fn create_voter(
        ctx: Context<CreateVoter>,
        voter_bump: u8,
        voter_weight_record_bump: u8,
    ) -> Result<()> {
        instructions::create_voter(ctx, voter_bump, voter_weight_record_bump)
    }

    pub fn create_deposit_entry(
        ctx: Context<CreateDepositEntry>,
        deposit_entry_index: u8,
        kind: LockupKind,
        period: LockupPeriod,
    ) -> Result<()> {
        instructions::create_deposit_entry(ctx, deposit_entry_index, kind, period)
    }

    pub fn deposit(ctx: Context<Deposit>, deposit_entry_index: u8, amount: u64) -> Result<()> {
        instructions::deposit(ctx, deposit_entry_index, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>, deposit_entry_index: u8, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, deposit_entry_index, amount)
    }

    pub fn close_deposit_entry(
        ctx: Context<CloseDepositEntry>,
        deposit_entry_index: u8,
    ) -> Result<()> {
        instructions::close_deposit_entry(ctx, deposit_entry_index)
    }

    pub fn update_voter_weight_record(ctx: Context<UpdateVoterWeightRecord>) -> Result<()> {
        instructions::update_voter_weight_record(ctx)
    }

    pub fn unlock_tokens(ctx: Context<Stake>, deposit_entry_index: u8) -> Result<()> {
        instructions::unlock_tokens(ctx, deposit_entry_index)
    }

    pub fn close_voter<'info>(ctx: Context<'_, '_, '_, 'info, CloseVoter<'info>>) -> Result<()> {
        instructions::close_voter(ctx)
    }

    pub fn log_voter_info(
        ctx: Context<LogVoterInfo>,
        deposit_entry_begin: u8,
        deposit_entry_count: u8,
    ) -> Result<()> {
        instructions::log_voter_info(ctx, deposit_entry_begin, deposit_entry_count)
    }

    pub fn stake(
        ctx: Context<Stake>,
        source_deposit_entry_index: u8,
        target_deposit_entry_index: u8,
        amount: u64,
    ) -> Result<()> {
        instructions::stake(
            ctx,
            source_deposit_entry_index,
            target_deposit_entry_index,
            amount,
        )
    }

    pub fn extend_stake(
        ctx: Context<Stake>,
        source_deposit_entry_index: u8,
        target_deposit_entry_index: u8,
        new_lockup_period: LockupPeriod,
        additional_amount: u64,
    ) -> Result<()> {
        instructions::extend_stake(
            ctx,
            source_deposit_entry_index,
            target_deposit_entry_index,
            new_lockup_period,
            additional_amount,
        )
    }

    pub fn claim(
        ctx: Context<Claim>,
        registrar_bump: u8,
        realm_governing_mint_pubkey: Pubkey,
        realm_pubkey: Pubkey,
    ) -> Result<u64> {
        instructions::claim(
            ctx,
            registrar_bump,
            realm_governing_mint_pubkey,
            realm_pubkey,
        )
    }

    pub fn change_delegate(ctx: Context<ChangeDelegate>, deposit_entry_index: u8) -> Result<()> {
        instructions::change_delegate(ctx, deposit_entry_index)
    }
}

#[derive(Accounts)]
pub struct Stake<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.voter_bump,
        has_one = voter_authority,
        has_one = registrar)
    ]
    pub voter: AccountLoader<'info, Voter>,
    pub voter_authority: Signer<'info>,

    /// CHECK: delegate might be any arbitrary address
    pub delegate: UncheckedAccount<'info>,

    /// CHECK: Mining Account that belongs to Rewards Program and some delegate
    /// The address of the mining account on the rewards progra,
    /// derived from PDA(["mining", delegate wallet addr, reward_pool], rewards_program)
    #[account(mut)]
    pub delegate_mining: UncheckedAccount<'info>,

    /// CHECK:
    /// Ownership of the account will be checked in the rewards contract
    /// It's the core account for the rewards contract, which will
    /// keep track of all rewards and staking logic.
    #[account(mut)]
    pub reward_pool: UncheckedAccount<'info>,

    /// CHECK: mining PDA will be checked in the rewards contract
    /// PDA(["mining", mining owner <aka voter_authority in our case>, reward_pool],
    /// reward_program)
    #[account(mut)]
    pub deposit_mining: UncheckedAccount<'info>,

    /// CHECK: Rewards Program account
    #[account(executable)]
    pub rewards_program: UncheckedAccount<'info>,
}

impl Stake<'_> {
    pub fn verify_delegate_and_its_mining(&self, deposit_entry: &DepositEntry) -> Result<()> {
        // check whether target delegate mining is the same as delegate mining from passed context
        require_eq!(
            deposit_entry.delegate,
            *self.delegate.to_account_info().key,
            MplStakingError::InvalidDelegate
        );

        let (calculated_delegate_mining, _) = find_mining_address(
            &self.rewards_program.to_account_info().key(),
            &self.delegate.to_account_info().key(),
            &self.reward_pool.key(),
        );
        require_eq!(
            calculated_delegate_mining,
            self.delegate_mining.to_account_info().key(),
            MplStakingError::InvalidMining
        );

        Ok(())
    }
}
