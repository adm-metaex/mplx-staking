use anchor_spl::token::TokenAccount;
use assert_custom_on_chain_error::AssertCustomOnChainErr;
use mplx_staking_states::{
    error::MplStakingError,
    state::{LockupKind, LockupPeriod},
};
use program_test::*;
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transport::TransportError};

mod program_test;

struct Balances {
    token: u64,
    vault: u64,
    deposit: u64,
    voter_weight: u64,
}

async fn balances(
    context: &TestContext,
    registrar: &RegistrarCookie,
    address: Pubkey,
    voter: &VoterCookie,
    voting_mint: &VotingMintConfigCookie,
    deposit_id: u8,
) -> Balances {
    // Advance slots to avoid caching of the UpdateVoterWeightRecord call
    // TODO: Is this something that could be an issue on a live node?
    context.solana.advance_clock_by_slots(2).await;

    let token = context.solana.token_account_balance(address).await;
    let vault = voting_mint.vault_balance(&context.solana, voter).await;
    let deposit = voter.deposit_amount(&context.solana, deposit_id).await;
    let vwr = context
        .addin
        .update_voter_weight_record(registrar, voter)
        .await
        .unwrap();
    Balances {
        token,
        vault,
        deposit,
        voter_weight: vwr.voter_weight,
    }
}

#[tokio::test]
async fn test_deposit_constant() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let addin = &context.addin;

    let payer = &context.users[0].key;
    let realm_authority = Keypair::new();
    let realm = context
        .governance
        .create_realm(
            "testrealm",
            realm_authority.pubkey(),
            &context.mints[0],
            payer,
            &context.addin.program_id,
        )
        .await;

    let deposit_authority = &context.users[1].key;
    let token_owner_record = realm
        .create_token_owner_record(deposit_authority.pubkey(), payer)
        .await;

    let fill_authority = Keypair::from_bytes(&context.users[3].key.to_bytes()).unwrap();
    let distribution_authority = Keypair::new();
    let (registrar, rewards_pool) = context
        .addin
        .create_registrar(
            &realm,
            &realm_authority,
            payer,
            &fill_authority.pubkey(),
            &distribution_authority.pubkey(),
            &context.rewards.program_id,
        )
        .await;
    let mngo_voting_mint = addin
        .configure_voting_mint(
            &registrar,
            &realm_authority,
            payer,
            0,
            &context.mints[0],
            None,
            None,
        )
        .await;

    // TODO: ??? voter_authority == deposit_authority ???
    let voter_authority = deposit_authority;
    let (deposit_mining, _) = find_deposit_mining_addr(
        &context.rewards.program_id,
        &voter_authority.pubkey(),
        &rewards_pool,
    );

    let voter = addin
        .create_voter(
            &registrar,
            &token_owner_record,
            voter_authority,
            payer,
            &rewards_pool,
            &deposit_mining,
            &context.rewards.program_id,
        )
        .await;

    let reference_account = context.users[1].token_accounts[0];
    let get_balances = |depot_id| {
        balances(
            &context,
            &registrar,
            reference_account,
            &voter,
            &mngo_voting_mint,
            depot_id,
        )
    };
    let withdraw = |amount: u64, d_entry_index: u8| {
        addin.withdraw(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            d_entry_index,
            amount,
        )
    };
    let deposit = |amount: u64, d_entry_index: u8| {
        addin.deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            d_entry_index,
            amount,
        )
    };

    // test deposit and withdraw
    let token = context
        .solana
        .token_account_balance(reference_account)
        .await;

    addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await
        .unwrap();
    addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::ThreeMonths,
        )
        .await
        .unwrap();
    deposit(10_000, 0).await.unwrap();
    addin
        .stake(
            &registrar,
            &voter,
            voter.authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            10000,
        )
        .await?;

    let after_deposit = get_balances(1).await;
    assert_eq!(token, after_deposit.token + after_deposit.vault);
    assert_eq!(after_deposit.voter_weight, after_deposit.vault); // unchanged
    assert_eq!(after_deposit.vault, 10_000);
    assert_eq!(after_deposit.deposit, 10_000);
    withdraw(1, 1)
        .await
        .assert_on_chain_err(MplStakingError::UnlockMustBeCalledFirst);

    // advance to day 95. Just to be sure withdraw isn't possible without unlocking first
    // even at lockup period + cooldown period (90 + 5 respectively in that case)
    let secs_per_day = 24 * 60 * 60;
    addin
        .set_time_offset(&registrar, &realm_authority, secs_per_day * 95)
        .await;

    // request unlock
    addin
        .unlock_tokens(
            &registrar,
            &voter,
            &voter,
            1,
            &rewards_pool,
            &context.rewards.program_id,
        )
        .await
        .unwrap();
    withdraw(10_000, 1)
        .await
        .assert_on_chain_err(MplStakingError::InvalidTimestampArguments);

    context.solana.advance_clock_by_slots(2).await; // avoid caching of transactions
                                                    // warp to day 100. (90 days of lockup + fake cooldown (5 days)) + 5 days of true cooldown
    addin
        .set_time_offset(&registrar, &realm_authority, secs_per_day * 100)
        .await;

    // request claim && withdraw
    withdraw(10_000, 1).await.unwrap();

    Ok(())
}

#[tokio::test]
async fn test_withdrawing_without_unlocking() -> Result<(), TransportError> {
    let context = TestContext::new().await;
    let addin = &context.addin;

    let payer = &context.users[0].key;
    let realm_authority = Keypair::new();
    let realm = context
        .governance
        .create_realm(
            "testrealm",
            realm_authority.pubkey(),
            &context.mints[0],
            payer,
            &context.addin.program_id,
        )
        .await;

    let voter_authority = &context.users[1].key;
    let token_owner_record = realm
        .create_token_owner_record(voter_authority.pubkey(), payer)
        .await;

    let fill_authority = Keypair::from_bytes(&context.users[3].key.to_bytes()).unwrap();
    let distribution_authority = Keypair::new();
    let (registrar, rewards_pool) = context
        .addin
        .create_registrar(
            &realm,
            &realm_authority,
            payer,
            &fill_authority.pubkey(),
            &distribution_authority.pubkey(),
            &context.rewards.program_id,
        )
        .await;
    let mngo_voting_mint = addin
        .configure_voting_mint(
            &registrar,
            &realm_authority,
            payer,
            0,
            &context.mints[0],
            None,
            None,
        )
        .await;

    let (deposit_mining, _) = find_deposit_mining_addr(
        &context.rewards.program_id,
        &voter_authority.pubkey(),
        &rewards_pool,
    );

    let voter = addin
        .create_voter(
            &registrar,
            &token_owner_record,
            voter_authority,
            payer,
            &rewards_pool,
            &deposit_mining,
            &context.rewards.program_id,
        )
        .await;

    let reference_account = context.users[1].token_accounts[0];
    let withdraw = |amount: u64| {
        addin.withdraw(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            0,
            amount,
        )
    };
    let deposit = |amount: u64, d_entry_index: u8| {
        addin.deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            d_entry_index,
            amount,
        )
    };

    addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await
        .unwrap();
    addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::ThreeMonths,
        )
        .await
        .unwrap();
    deposit(10000, 0).await.unwrap();
    addin
        .stake(
            &registrar,
            &voter,
            voter.authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            10000,
        )
        .await?;

    // advance to 100 days
    addin
        .set_time_offset(&registrar, &realm_authority, 100 * 24 * 60 * 60)
        .await;

    // withdraw
    withdraw(10_000)
        .await
        .assert_on_chain_err(MplStakingError::InsufficientUnlockedTokens);

    Ok(())
}
