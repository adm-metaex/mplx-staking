use crate::{borsh::BorshDeserialize, cpi_instructions};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use mplx_staking_states::error::MplStakingError;
use solana_program::program::get_return_data;
use std::borrow::Borrow;

#[derive(Accounts)]
pub struct Claim<'info> {
    /// CHECK: Reward Pool PDA will be checked in the rewards contract
    /// PDA(["reward_pool", deposit_authority[aka registrar in our case]], rewards_program)
    pub reward_pool: UncheckedAccount<'info>,

    /// CHECK: Rewards mint addr will be checked in the rewards contract
    pub reward_mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Rewards vault is used as a source of rewards and
    /// is checked on the rewards contract
    /// PDA(["vault", reward_pool, reward_mint], reward_program)
    pub vault: UncheckedAccount<'info>,

    /// CHECK: mining PDA will be checked in the rewards contract
    /// PDA(["mining", mining owner <aka voter_authority in our case>, reward_pool],
    /// reward_program)
    #[account(mut)]
    pub deposit_mining: UncheckedAccount<'info>,

    pub mining_owner: Signer<'info>,

    /// CHECK: Registrar plays the role of deposit_authority on the Rewards Contract,
    /// therefore their PDA that should sign the CPI call
    pub registrar: UncheckedAccount<'info>,

    #[account(mut)]
    pub user_reward_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,

    /// CHECK: Rewards Program account
    #[account(executable)]
    pub rewards_program: UncheckedAccount<'info>,
}

/// Claims token from the Rewards Contract.
///
/// Tokens will be transfered from Vault in Rewards account to User's user_reward_token_account.
/// This call actually doesn't mutating Staking's accounts, only Reward's accounts will be mutated.
pub fn claim(
    ctx: Context<Claim>,
    registrar_bump: u8,
    realm_governing_mint_pubkey: Pubkey,
    realm_pubkey: Pubkey,
) -> Result<u64> {
    let rewards_program = ctx.accounts.rewards_program.to_account_info();
    let reward_pool = ctx.accounts.reward_pool.to_account_info();
    let rewards_mint = ctx.accounts.reward_mint.to_account_info();
    let vault = ctx.accounts.vault.to_account_info();
    let deposit_mining = ctx.accounts.deposit_mining.to_account_info();
    let deposit_authority = ctx.accounts.registrar.to_account_info();
    let mining_owner = ctx.accounts.mining_owner.to_account_info();
    let user_reward_token_account = ctx.accounts.user_reward_token_account.to_account_info();
    let token_program = ctx.accounts.token_program.to_account_info();
    let signers_seeds = &[
        &realm_pubkey.key().to_bytes(),
        b"registrar".as_ref(),
        &realm_governing_mint_pubkey.key().to_bytes(),
        &[registrar_bump][..],
    ];

    cpi_instructions::claim(
        rewards_program,
        reward_pool,
        rewards_mint,
        vault,
        deposit_mining,
        mining_owner,
        deposit_authority,
        user_reward_token_account,
        token_program,
        signers_seeds,
    )?;

    if let Some((_rewards_program_id, claimed_rewards_raw)) = get_return_data() {
        let claimed_rewards = u64::deserialize(&mut claimed_rewards_raw.borrow())?;
        msg!("Rewards are clamed {:?}", claimed_rewards);
        Ok(claimed_rewards)
    } else {
        Err(MplStakingError::CpiReturnDataIsAbsent.into())
    }
}
