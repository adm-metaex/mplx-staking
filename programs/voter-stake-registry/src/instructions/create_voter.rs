use crate::{cpi_instructions, voter::VoterWeightRecord};
use anchor_lang::prelude::*;
use mplx_staking_states::{
    error::MplStakingError,
    registrar_seeds,
    state::{Registrar, Voter},
};
use solana_program::instruction::{get_stack_height, TRANSACTION_LEVEL_STACK_HEIGHT};
use std::mem::size_of;

#[derive(Accounts)]
pub struct CreateVoter<'info> {
    /// Also, Registrar plays the role of deposit_authority on the Rewards Contract,
    /// therefore their PDA that should sign the CPI call
    pub registrar: AccountLoader<'info, Registrar>,

    #[account(
        init,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + size_of::<Voter>(),
    )]
    pub voter: AccountLoader<'info, Voter>,

    /// The authority controling the voter. Must be the same as the
    /// `governing_token_owner` in the token owner record used with
    /// spl-governance.
    pub voter_authority: Signer<'info>,

    /// The voter weight record is the account that will be shown to spl-governance
    /// to prove how much vote weight the voter has. See update_voter_weight_record.
    #[account(
        init,
        seeds = [registrar.key().as_ref(), b"voter-weight-record".as_ref(), voter_authority.key().as_ref()],
        bump,
        payer = payer,
        space = size_of::<VoterWeightRecord>(),
    )]
    pub voter_weight_record: Box<Account<'info, VoterWeightRecord>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK:
    /// Ownership of the account will be checked in the rewards contract
    /// It's the core account for the rewards contract, which will
    /// keep track of all rewards and staking logic.
    #[account(mut)]
    pub reward_pool: UncheckedAccount<'info>,

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

    /// CHECK: Rewards program ID
    #[account(executable)]
    pub rewards_program: UncheckedAccount<'info>,
}

/// Creates a new voter account. There can only be a single voter per
/// voter_authority.
///
/// The user must register with spl-governance using the same voter_authority.
/// Their token owner record will be required for withdrawing funds later.
pub fn create_voter(
    ctx: Context<CreateVoter>,
    voter_bump: u8,
    voter_weight_record_bump: u8,
) -> Result<()> {
    {
        // The current stack height must be the initial one. Otherwise, it's a CPI.
        if get_stack_height() > TRANSACTION_LEVEL_STACK_HEIGHT {
            return err!(MplStakingError::ForbiddenCpi);
        }

        require_eq!(voter_bump, *ctx.bumps.get("voter").unwrap());
        require_eq!(
            voter_weight_record_bump,
            *ctx.bumps.get("voter_weight_record").unwrap()
        );

        // Load accounts.
        let registrar = ctx.accounts.registrar.load()?;

        require!(
            ctx.accounts.rewards_program.key() == registrar.rewards_program,
            MplStakingError::InvalidRewardsProgram
        );

        require!(
            registrar.reward_pool == ctx.accounts.reward_pool.key(),
            MplStakingError::InvalidRewardPool
        );

        let voter_authority = ctx.accounts.voter_authority.key();

        let voter = &mut ctx.accounts.voter.load_init()?;
        voter.voter_bump = voter_bump;
        voter.voter_weight_record_bump = voter_weight_record_bump;
        voter.voter_authority = voter_authority;
        voter.authorized_agent = voter_authority;
        voter.registrar = ctx.accounts.registrar.key();

        let voter_weight_record = &mut ctx.accounts.voter_weight_record;
        voter_weight_record.account_discriminator =
            spl_governance_addin_api::voter_weight::VoterWeightRecord::ACCOUNT_DISCRIMINATOR;
        voter_weight_record.realm = registrar.realm;
        voter_weight_record.governing_token_mint = registrar.realm_governing_token_mint;
        voter_weight_record.governing_token_owner = voter_authority;
    }

    // initialize Mining account for Voter
    let registrar = ctx.accounts.registrar.load()?;
    let signer_seeds = registrar_seeds!(registrar);

    let mining = ctx.accounts.deposit_mining.to_account_info();
    let payer = ctx.accounts.payer.to_account_info();
    let user = ctx.accounts.voter_authority.key;
    let deposit_authority = ctx.accounts.registrar.to_account_info();
    let system_program = ctx.accounts.system_program.to_account_info();
    let reward_pool = ctx.accounts.reward_pool.to_account_info();
    let rewards_program_id = ctx.accounts.rewards_program.to_account_info();

    cpi_instructions::initialize_mining(
        rewards_program_id,
        reward_pool,
        mining,
        user,
        payer,
        deposit_authority,
        system_program,
        signer_seeds,
    )?;

    Ok(())
}
