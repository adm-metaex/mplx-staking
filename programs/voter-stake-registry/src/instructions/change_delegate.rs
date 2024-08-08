use crate::{
    clock_unix_timestamp, cpi_instructions, find_mining_address, MplStakingError, Registrar, Voter,
};
use anchor_lang::prelude::*;
use solana_program::clock::SECONDS_PER_DAY;

pub const DELEGATE_UPDATE_DIFF_THRESHOLD: u64 = 5 * SECONDS_PER_DAY;

#[derive(Accounts)]
pub struct ChangeDelegate<'info> {
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

    pub delegate_voter: AccountLoader<'info, Voter>,

    /// CHECK: Mining Account that belongs to Rewards Program and some delegate
    /// The address of the mining account on the rewards program
    /// derived from PDA(["mining", delegate wallet addr, reward_pool], rewards_program)
    /// Seeds derivation will be checked on the rewards contract
    #[account(mut)]
    pub old_delegate_mining: UncheckedAccount<'info>,

    /// CHECK: Mining Account that belongs to Rewards Program and some delegate
    /// The address of the mining account on the rewards program
    /// derived from PDA(["mining", delegate wallet addr, reward_pool], rewards_program)
    /// Seeds derivation will be checked on the rewards contract
    #[account(mut)]
    pub new_delegate_mining: UncheckedAccount<'info>,

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

/// Changes delegate for the existing stake.
///
/// Rewards will be recalculated, and the new delegate will start receiving rewards.
/// The old delegate will stop receiving rewards.
/// It might be done once per five days.
pub fn change_delegate(ctx: Context<ChangeDelegate>, deposit_entry_index: u8) -> Result<()> {
    let registrar = &ctx.accounts.registrar.load()?;
    let voter = &mut ctx.accounts.voter.load_mut()?;
    let voter_authority = voter.voter_authority;
    let target = voter.active_deposit_mut(deposit_entry_index)?;
    let curr_ts = clock_unix_timestamp();

    require!(
        registrar.reward_pool == ctx.accounts.reward_pool.key(),
        MplStakingError::InvalidRewardPool
    );

    let delegate_last_update_diff = curr_ts
        .checked_sub(target.delegate_last_update_ts)
        .ok_or(MplStakingError::ArithmeticOverflow)?;

    require!(
        delegate_last_update_diff > DELEGATE_UPDATE_DIFF_THRESHOLD,
        MplStakingError::DelegateUpdateIsTooSoon
    );

    if ctx.accounts.voter.key() == ctx.accounts.delegate_voter.key() {
        require!(
            target.delegate != voter_authority,
            MplStakingError::SameDelegate
        );
        target.delegate = voter_authority;
    } else {
        let delegate_voter = &ctx.accounts.delegate_voter.load()?;
        require!(
            ctx.accounts.voter.key() != ctx.accounts.delegate_voter.key()
                && delegate_voter.voter_authority != target.delegate,
            MplStakingError::SameDelegate
        );

        let delegate_voter_weighted_stake = delegate_voter
            .deposits
            .iter()
            .fold(0, |acc, d| acc + d.weighted_stake(curr_ts));
        require!(
            delegate_voter_weighted_stake >= Voter::MIN_OWN_WEIGHTED_STAKE,
            MplStakingError::InsufficientWeightedStake
        );

        let (delegate_mining, _) = find_mining_address(
            &ctx.accounts.rewards_program.key(),
            &delegate_voter.voter_authority,
            &ctx.accounts.reward_pool.key(),
        );

        require!(
            delegate_mining == ctx.accounts.new_delegate_mining.key(),
            MplStakingError::InvalidMining
        );
        target.delegate = delegate_voter.voter_authority;
        target.delegate_last_update_ts = curr_ts;
    }

    let reward_pool = ctx.accounts.reward_pool.to_account_info();
    let mining = ctx.accounts.deposit_mining.to_account_info();
    let deposit_authority = ctx.accounts.registrar.to_account_info();
    let old_delegate_mining = ctx.accounts.old_delegate_mining.to_account_info();
    let new_delegate_mining = ctx.accounts.new_delegate_mining.to_account_info();
    let signers_seeds = &[
        &registrar.realm.key().to_bytes(),
        b"registrar".as_ref(),
        &registrar.realm_governing_token_mint.key().to_bytes(),
        &[registrar.bump][..],
    ];
    let staked_amount = target.amount_deposited_native;
    let mining_owner = ctx.accounts.voter_authority.to_account_info();

    cpi_instructions::change_delegate(
        ctx.accounts.rewards_program.to_account_info(),
        reward_pool,
        mining,
        deposit_authority,
        mining_owner,
        old_delegate_mining,
        new_delegate_mining,
        staked_amount,
        signers_seeds,
    )?;

    Ok(())
}
