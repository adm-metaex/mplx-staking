use anchor_spl::token::TokenAccount;
use assert_custom_on_chain_error::AssertCustomOnChainErr;
use mplx_staking_states::{
    error::MplStakingError,
    state::{LockupKind, LockupPeriod},
};
use program_test::*;
use solana_program_test::*;
use solana_sdk::{
    clock::SECONDS_PER_DAY, signature::Keypair, signer::Signer, transport::TransportError,
};

mod program_test;

#[tokio::test]
async fn extend_from_flex() -> Result<(), TransportError> {
    let context = TestContext::new().await;

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
    context
        .addin
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
    let mngo_voting_mint = context
        .addin
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

    let voter = context
        .addin
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

    // test deposit and withdraw
    let reference_account = context.users[1].token_accounts[0];
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::OneYear,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            0,
            10000,
        )
        .await?;
    context
        .addin
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
    advance_clock_by_ts(
        &mut context.solana.context.borrow_mut(),
        (SECONDS_PER_DAY * 365) as i64,
    )
    .await;

    context
        .addin
        .extend_stake(
            &registrar,
            &voter,
            voter_authority,
            &voter_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            LockupPeriod::OneYear,
            0,
        )
        .await?;

    let vault_balance = mngo_voting_mint
        .vault_balance(&context.solana, &voter)
        .await;
    let deposit_amount = voter.deposit_amount(&context.solana, 1).await;

    assert_eq!(vault_balance, 10000);
    assert_eq!(deposit_amount, 10000);

    Ok(())
}

#[tokio::test]
async fn extend_from_three_months_deposit() -> Result<(), TransportError> {
    let context = TestContext::new().await;

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
    context
        .addin
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
    let mngo_voting_mint = context
        .addin
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

    let voter = context
        .addin
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

    // test deposit and withdraw
    let reference_account = context.users[1].token_accounts[0];
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::OneYear,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            0,
            10000,
        )
        .await?;
    context
        .addin
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
    advance_clock_by_ts(
        &mut context.solana.context.borrow_mut(),
        (SECONDS_PER_DAY * 365) as i64,
    )
    .await;

    context
        .addin
        .extend_stake(
            &registrar,
            &voter,
            voter_authority,
            &voter_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            LockupPeriod::OneYear,
            0,
        )
        .await?;

    let vault_balance = mngo_voting_mint
        .vault_balance(&context.solana, &voter)
        .await;
    let deposit_amount = voter.deposit_amount(&context.solana, 1).await;

    assert_eq!(vault_balance, 10000);
    assert_eq!(deposit_amount, 10000);

    Ok(())
}

#[tokio::test]
async fn extend_deposit_after_one_year_for_three_months_with_top_up() -> Result<(), TransportError>
{
    let context = TestContext::new().await;

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
    context
        .addin
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
    let mngo_voting_mint = context
        .addin
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

    let voter = context
        .addin
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

    // test deposit and withdraw
    let reference_account = context.users[1].token_accounts[0];
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::OneYear,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            0,
            10500,
        )
        .await?;
    context
        .addin
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
    advance_clock_by_ts(
        &mut context.solana.context.borrow_mut(),
        (SECONDS_PER_DAY * 365) as i64,
    )
    .await;

    context
        .addin
        .extend_stake(
            &registrar,
            &voter,
            voter_authority,
            &voter_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            LockupPeriod::ThreeMonths,
            500,
        )
        .await?;

    let vault_balance = mngo_voting_mint
        .vault_balance(&context.solana, &voter)
        .await;
    let deposit_amount = voter.deposit_amount(&context.solana, 1).await;

    assert_eq!(vault_balance, 10500);
    assert_eq!(deposit_amount, 10500);

    Ok(())
}

#[tokio::test]
async fn extend_from_flex_deposit_with_top_up() -> Result<(), TransportError> {
    let context = TestContext::new().await;

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
    context
        .addin
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
    let mngo_voting_mint = context
        .addin
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

    let voter = context
        .addin
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

    // test deposit and withdraw
    let reference_account = context.users[1].token_accounts[0];
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::OneYear,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            0,
            10500,
        )
        .await?;
    context
        .addin
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
    advance_clock_by_ts(
        &mut context.solana.context.borrow_mut(),
        (SECONDS_PER_DAY * 365) as i64,
    )
    .await;

    context
        .addin
        .extend_stake(
            &registrar,
            &voter,
            voter_authority,
            &voter_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            LockupPeriod::OneYear,
            500,
        )
        .await?;

    let deposit_amount = voter.deposit_amount(&context.solana, 1).await;

    assert_eq!(deposit_amount, 10500);

    Ok(())
}

#[tokio::test]
async fn extend_from_three_month_to_one_year() -> Result<(), TransportError> {
    let context = TestContext::new().await;

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
    context
        .addin
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
    let mngo_voting_mint = context
        .addin
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

    let voter = context
        .addin
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

    // test deposit and withdraw
    let reference_account = context.users[1].token_accounts[0];
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::ThreeMonths,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            0,
            10000,
        )
        .await?;
    context
        .addin
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
    advance_clock_by_ts(
        &mut context.solana.context.borrow_mut(),
        (SECONDS_PER_DAY * 10) as i64,
    )
    .await;

    context
        .addin
        .extend_stake(
            &registrar,
            &voter,
            voter_authority,
            &voter_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            LockupPeriod::OneYear,
            50,
        )
        .await
        .assert_on_chain_err(MplStakingError::ArithmeticOverflow);

    Ok(())
}

#[tokio::test]
async fn prolongs_with_delegate() -> Result<(), TransportError> {
    let context = TestContext::new().await;

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
    context
        .addin
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
    let mngo_voting_mint = context
        .addin
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

    // CREATE DELEGATE
    let delegate_authority = &context.users[2].key;
    let delegate_token_account = context.users[2].token_accounts[0];

    let (delegate_mining, _) = find_deposit_mining_addr(
        &context.rewards.program_id,
        &delegate_authority.pubkey(),
        &rewards_pool,
    );

    let delegate_voter = context
        .addin
        .create_voter(
            &registrar,
            &token_owner_record,
            delegate_authority,
            payer,
            &rewards_pool,
            &delegate_mining,
            &context.rewards.program_id,
        )
        .await;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &delegate_voter,
            &delegate_voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &delegate_voter,
            &delegate_voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::OneYear,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &delegate_voter,
            &mngo_voting_mint,
            delegate_authority,
            delegate_token_account,
            0,
            6_000_000,
        )
        .await?;
    context
        .addin
        .stake(
            &registrar,
            &delegate_voter,
            delegate_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            6_000_000,
        )
        .await?;

    // CREATE VOTER
    let voter = context
        .addin
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

    // test deposit and stake
    let voter_token_account = context.users[1].token_accounts[0];
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            0,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await?;
    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &delegate_voter,
            &mngo_voting_mint,
            1,
            LockupKind::Constant,
            LockupPeriod::OneYear,
        )
        .await?;
    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            voter_token_account,
            0,
            10000,
        )
        .await?;
    context
        .addin
        .stake(
            &registrar,
            &voter,
            delegate_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            10000,
        )
        .await?;

    context
        .addin
        .extend_stake(
            &registrar,
            &voter,
            voter_authority,
            &delegate_authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            LockupPeriod::OneYear,
            0,
        )
        .await?;

    let vault_balance = mngo_voting_mint
        .vault_balance(&context.solana, &voter)
        .await;
    let deposit_amount = voter.deposit_amount(&context.solana, 1).await;

    assert_eq!(vault_balance, 10000);
    assert_eq!(deposit_amount, 10000);

    Ok(())
}
