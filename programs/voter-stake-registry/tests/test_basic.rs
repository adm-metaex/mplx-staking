use anchor_spl::token::TokenAccount;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};

use mplx_staking_states::state::{LockupKind, LockupPeriod};
use program_test::*;

mod program_test;

#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
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
    context
        .addin
        .configure_voting_mint(
            &registrar,
            &realm_authority,
            payer,
            0,
            &context.mints[0],
            10,
            0.0,
            0.0,
            1,
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
            0,
            1.0,
            0.0,
            5 * 365 * 24 * 60 * 60,
            None,
            None,
        )
        .await;

    // TODO: ??? voter_authority == deposit_authority ???
    let voter_authority = deposit_authority;
    let deposit_mining = find_deposit_mining_addr(
        &voter_authority.pubkey(),
        &rewards_pool,
        &context.rewards.program_id,
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
    let reference_initial = context
        .solana
        .token_account_balance(reference_account)
        .await;
    let balance_initial = voter.deposit_amount(&context.solana, 0).await;
    assert_eq!(balance_initial, 0);

    context
        .addin
        .create_deposit_entry(
            &registrar,
            &voter,
            voter_authority,
            &mngo_voting_mint,
            0,
            LockupKind::Constant,
            None,
            LockupPeriod::ThreeMonths,
        )
        .await?;

    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            deposit_authority,
            reference_account,
            0,
            10000,
            &rewards_pool,
            &deposit_mining,
            &context.rewards.program_id,
        )
        .await?;

    let reference_after_deposit = context
        .solana
        .token_account_balance(reference_account)
        .await;
    assert_eq!(reference_initial, reference_after_deposit + 10000);
    let vault_after_deposit = mngo_voting_mint
        .vault_balance(&context.solana, &voter)
        .await;
    assert_eq!(vault_after_deposit, 10000);
    let balance_after_deposit = voter.deposit_amount(&context.solana, 0).await;
    assert_eq!(balance_after_deposit, 10000);

    context
        .addin
        .set_time_offset(&registrar, &realm_authority, 100 * 86400)
        .await;

    context
        .addin
        .unlock_tokens(&registrar, &voter, voter_authority, 0)
        .await
        .unwrap();

    context
        .addin
        .set_time_offset(&registrar, &realm_authority, 106 * 86400)
        .await;

    context
        .addin
        .withdraw(
            &registrar,
            &voter,
            &mngo_voting_mint,
            deposit_authority,
            reference_account,
            0,
            10000,
            &rewards_pool,
            &deposit_mining,
            &context.rewards.program_id,
        )
        .await?;

    let reference_after_withdraw = context
        .solana
        .token_account_balance(reference_account)
        .await;
    assert_eq!(reference_initial, reference_after_withdraw);
    let vault_after_withdraw = mngo_voting_mint
        .vault_balance(&context.solana, &voter)
        .await;
    assert_eq!(vault_after_withdraw, 0);
    let balance_after_withdraw = voter.deposit_amount(&context.solana, 0).await;
    assert_eq!(balance_after_withdraw, 0);

    let lamports_before = context
        .solana
        .context
        .borrow_mut()
        .banks_client
        .get_balance(voter_authority.pubkey())
        .await?;
    context
        .addin
        .close_voter(&registrar, &voter, &mngo_voting_mint, voter_authority)
        .await?;
    let lamports_after = context
        .solana
        .context
        .borrow_mut()
        .banks_client
        .get_balance(voter_authority.pubkey())
        .await?;
    assert!(lamports_after > lamports_before);

    Ok(())
}
