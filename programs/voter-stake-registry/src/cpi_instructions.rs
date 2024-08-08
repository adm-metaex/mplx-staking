use crate::state::LockupPeriod;
use anchor_lang::{prelude::*, Key};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
    system_program,
};

pub const REWARD_CONTRACT_ID: Pubkey =
    solana_program::pubkey!("BF5PatmRTQDgEKoXR7iHRbkibEEi83nVM38cUKWzQcTR");

#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Eq)]

pub enum RewardsInstruction {
    /// Creates and initializes a reward pool account
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [R] Reward mint account
    /// [W] Vault account
    /// [WS] Payer
    /// [RS] Deposit authority account
    /// [R] Rent sysvar
    /// [R] Token program
    /// [R] System program
    InitializePool {
        /// Account can fill the reward vault
        fill_authority: Pubkey,
        /// Account can distribute rewards for stakers
        distribution_authority: Pubkey,
    },

    /// Fills the reward pool with rewards
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [R] Mint of rewards account
    /// [W] Vault for rewards account
    /// [RS] Transfer  account
    /// [W] From account
    /// [R] Token program
    FillVault {
        /// Amount to fill
        amount: u64,
        /// Rewards distribution ends at given date
        distribution_ends_at: u64,
    },

    /// Initializes mining account for the specified mining owner
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [W] Mining
    /// [WS] Payer
    /// [R] System program
    InitializeMining {
        /// Represent the end-user, owner of the mining
        mining_owner: Pubkey,
    },

    /// Deposits amount of supply to the mining account
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [W] Mining
    /// [R] Mint of rewards account
    /// [RS] Deposit authority
    DepositMining {
        /// Amount to deposit
        amount: u64,
        /// Lockup Period
        lockup_period: LockupPeriod,
        /// Specifies the owner of the Mining Account
        owner: Pubkey,
    },

    /// Withdraws amount of supply to the mining account
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [W] Mining
    /// [R] Mining owner
    /// [RS] Deposit authority
    WithdrawMining {
        /// Amount to withdraw
        amount: u64,
        /// Specifies the owner of the Mining Account
        owner: Pubkey,
    },

    /// Claims amount of rewards
    ///
    /// Accounts:
    /// [R] Reward pool account
    /// [R] Mint of rewards account
    /// [W] Vault for rewards account
    /// [W] Mining
    /// [RS] Mining owner
    /// [RS] Deposit authority
    /// [W] Mining owner reward token account
    /// [R] Token program
    Claim,

    /// Extends stake
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [W] Mining
    /// [R] Mint of rewards account
    /// [R] Mining owner
    /// [RS] Deposit authority
    ExtendStake {
        /// Lockup period before restaking. Actually it's only needed
        /// for Flex to AnyPeriod edge case
        old_lockup_period: LockupPeriod,
        /// Requested lockup period for restaking
        new_lockup_period: LockupPeriod,
        /// Deposit start_ts
        deposit_start_ts: u64,
        /// Amount of tokens to be restaked, this
        /// number cannot be decreased. It reflects the number of staked tokens
        /// before the extend_stake function call
        base_amount: u64,
        /// In case user wants to increase it's staked number of tokens,
        /// the addition amount might be provided
        additional_amount: u64,
        /// The wallet who owns the mining account
        mining_owner: Pubkey,
    },

    /// Distributes tokens among mining owners
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [R] Mint of rewards account
    /// [W] Vault for rewards account
    /// [RS] Distribute rewards authority
    DistributeRewards,

    /// Closes Mining Account
    CloseMining,

    /// Changes delegate for the existing stake
    ///
    /// Accounts:
    /// [W] Reward pool account
    /// [W] Mining
    /// [RS] Deposit authority
    /// [RS] Mining owner
    /// [W] Old delegate mining
    /// [W] New delegate mining
    ChangeDelegate {
        /// Amount of staked tokens
        staked_amount: u64,
    },
}

/// This function initializes pool. Some sort of a "root"
/// of the rewards contract
#[allow(clippy::too_many_arguments)]
pub fn initialize_pool<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    reward_mint: AccountInfo<'a>,
    reward_vault: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    fill_authority: Pubkey,
    distribution_authority: Pubkey,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(reward_pool.key(), false),
        AccountMeta::new_readonly(reward_mint.key(), false),
        AccountMeta::new(reward_vault.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new_readonly(rent.key(), false),
        AccountMeta::new_readonly(token_program.key(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let ix = Instruction::new_with_borsh(
        program_id.key(),
        &RewardsInstruction::InitializePool {
            fill_authority,
            distribution_authority,
        },
        accounts,
    );

    invoke_signed(
        &ix,
        &[
            reward_pool,
            reward_mint,
            reward_vault,
            payer,
            deposit_authority,
            rent,
            token_program,
            system_program,
            program_id,
        ],
        &[signers_seeds],
    )
}

/// Rewards initialize mining
#[allow(clippy::too_many_arguments)]
pub fn initialize_mining<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    mining_owner: &Pubkey,
    payer: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(reward_pool.key(), false),
        AccountMeta::new(mining.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let ix = Instruction::new_with_borsh(
        program_id.key(),
        &RewardsInstruction::InitializeMining {
            mining_owner: *mining_owner,
        },
        accounts,
    );

    invoke(
        &ix,
        &[reward_pool, mining, payer, system_program, program_id],
    )
}

/// Rewards deposit mining
#[allow(clippy::too_many_arguments)]
pub fn deposit_mining<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    delegate_mining: AccountInfo<'a>,
    amount: u64,
    lockup_period: LockupPeriod,
    owner: &Pubkey,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(reward_pool.key(), false),
        AccountMeta::new(mining.key(), false),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new(delegate_mining.key(), false),
    ];

    let ix = Instruction::new_with_borsh(
        program_id.key(),
        &RewardsInstruction::DepositMining {
            amount,
            lockup_period,
            owner: *owner,
        },
        accounts,
    );

    invoke_signed(
        &ix,
        &[
            reward_pool,
            mining,
            deposit_authority,
            delegate_mining,
            program_id,
        ],
        &[signers_seeds],
    )
}

/// Extend stake
#[allow(clippy::too_many_arguments)]
pub fn extend_stake<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    delegate_mining: AccountInfo<'a>,
    old_lockup_period: LockupPeriod,
    new_lockup_period: LockupPeriod,
    deposit_start_ts: u64,
    base_amount: u64,
    additional_amount: u64,
    mining_owner: &Pubkey,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(reward_pool.key(), false),
        AccountMeta::new(mining.key(), false),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new(delegate_mining.key(), false),
    ];

    let ix = Instruction::new_with_borsh(
        program_id.key(),
        &RewardsInstruction::ExtendStake {
            old_lockup_period,
            new_lockup_period,
            deposit_start_ts,
            base_amount,
            additional_amount,
            mining_owner: *mining_owner,
        },
        accounts,
    );

    invoke_signed(
        &ix,
        &[
            reward_pool,
            mining,
            deposit_authority,
            delegate_mining,
            program_id,
        ],
        &[signers_seeds],
    )?;

    Ok(())
}

/// Rewards withdraw mining
#[allow(clippy::too_many_arguments)]
pub fn withdraw_mining<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    delegate_mining: AccountInfo<'a>,
    amount: u64,
    owner: &Pubkey,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(reward_pool.key(), false),
        AccountMeta::new(mining.key(), false),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new(delegate_mining.key(), false),
    ];

    let ix = Instruction::new_with_borsh(
        program_id.key(),
        &RewardsInstruction::WithdrawMining {
            amount,
            owner: *owner,
        },
        accounts,
    );

    invoke_signed(
        &ix,
        &[
            reward_pool,
            mining,
            deposit_authority,
            delegate_mining,
            program_id,
        ],
        &[signers_seeds],
    )
}

/// Rewards withdraw mining
#[allow(clippy::too_many_arguments)]
pub fn claim<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    reward_mint: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    mining_owner: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    user_reward_token_account: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new_readonly(reward_pool.key(), false),
        AccountMeta::new_readonly(reward_mint.key(), false),
        AccountMeta::new(vault.key(), false),
        AccountMeta::new(mining.key(), false),
        AccountMeta::new_readonly(mining_owner.key(), true),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new(user_reward_token_account.key(), false),
        AccountMeta::new_readonly(token_program.key(), false),
    ];

    let ix = Instruction::new_with_borsh(program_id.key(), &RewardsInstruction::Claim, accounts);

    invoke_signed(
        &ix,
        &[
            reward_pool,
            reward_mint,
            vault,
            mining,
            mining_owner,
            deposit_authority,
            user_reward_token_account,
            token_program,
            program_id,
        ],
        &[signers_seeds],
    )
}

/// Rewards deposit mining
#[allow(clippy::too_many_arguments)]
pub fn close_mining<'a>(
    program_id: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    mining_owner: AccountInfo<'a>,
    target_account: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(mining.key(), false),
        AccountMeta::new_readonly(mining_owner.key(), true),
        AccountMeta::new(target_account.key(), false),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new_readonly(reward_pool.key(), false),
    ];

    let ix =
        Instruction::new_with_borsh(program_id.key(), &RewardsInstruction::CloseMining, accounts);

    invoke_signed(
        &ix,
        &[
            mining,
            mining_owner,
            target_account,
            deposit_authority,
            reward_pool,
            program_id,
        ],
        &[signers_seeds],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn change_delegate<'a>(
    program_id: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    mining_owner: AccountInfo<'a>,
    old_delegate_mining: AccountInfo<'a>,
    new_delegate_mining: AccountInfo<'a>,
    staked_amount: u64,
    signers_seeds: &[&[u8]],
) -> ProgramResult {
    let accounts = vec![
        AccountMeta::new(reward_pool.key(), false),
        AccountMeta::new(mining.key(), false),
        AccountMeta::new_readonly(deposit_authority.key(), true),
        AccountMeta::new_readonly(mining_owner.key(), true),
        AccountMeta::new(old_delegate_mining.key(), false),
        AccountMeta::new(new_delegate_mining.key(), false),
    ];

    let ix = Instruction::new_with_borsh(
        program_id.key(),
        &RewardsInstruction::ChangeDelegate { staked_amount },
        accounts,
    );

    invoke_signed(
        &ix,
        &[
            reward_pool,
            mining,
            deposit_authority,
            mining_owner,
            old_delegate_mining,
            new_delegate_mining,
            program_id,
        ],
        &[signers_seeds],
    )
}
