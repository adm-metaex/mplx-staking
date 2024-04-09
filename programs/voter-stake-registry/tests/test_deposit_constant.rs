use anchor_spl::token::TokenAccount;
use program_test::*;
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transport::TransportError};
use voter_stake_registry::state::{LockupKind, LockupPeriod};

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
    let vault = voting_mint.vault_balance(&context.solana, &voter).await;
    let deposit = voter.deposit_amount(&context.solana, deposit_id).await;
    let vwr = context
        .addin
        .update_voter_weight_record(&registrar, &voter)
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
            &payer,
            &context.addin.program_id,
        )
        .await;

    let voter_authority = &context.users[1].key;
    let token_owner_record = realm
        .create_token_owner_record(voter_authority.pubkey(), &payer)
        .await;

    let registrar = addin
        .create_registrar(&realm, &realm_authority, payer)
        .await;
    let mngo_voting_mint = addin
        .configure_voting_mint(
            &registrar,
            &realm_authority,
            payer,
            0,
            &context.mints[0],
            0,
            1.0,
            1.0,
            2 * 24 * 60 * 60,
            None,
            None,
        )
        .await;

    let voter = addin
        .create_voter(&registrar, &token_owner_record, &voter_authority, &payer)
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
    let withdraw = |amount: u64| {
        addin.withdraw(
            &registrar,
            &voter,
            &mngo_voting_mint,
            &voter_authority,
            reference_account,
            0,
            amount,
        )
    };
    let deposit = |amount: u64| {
        addin.deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            &voter_authority,
            reference_account,
            0,
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
            &voter_authority,
            &mngo_voting_mint,
            0,
            LockupKind::Constant,
            None,
            LockupPeriod::ThreeMonths,
        )
        .await
        .unwrap();
    deposit(10_000).await.unwrap();

    let after_deposit = get_balances(0).await;
    assert_eq!(token, after_deposit.token + after_deposit.vault);
    assert_eq!(after_deposit.voter_weight, after_deposit.vault); // unchanged
    assert_eq!(after_deposit.vault, 10_000);
    assert_eq!(after_deposit.deposit, 10_000);
    withdraw(1).await.expect_err("all locked up");

    // advance to day 92. Just to be sure withdraw isn't possible without unlocking first
    // even at lockup period + cooldown period (90 + 2 respectively in that case)
    let secs_per_day = 24 * 60 * 60;
    addin
        .set_time_offset(&registrar, &realm_authority, secs_per_day * 92)
        .await;

    // request unlock
    addin
        .unlock_tokens(&registrar, &voter, &voter_authority, 0)
        .await
        .unwrap();
    withdraw(10_000)
        .await
        .expect_err("Cooldown still not passed");

    context.solana.advance_clock_by_slots(2).await; // avoid caching of transactions
                                                    // warp to day 94. (92 days of lockup + fake cooldown) + 2 days of true cooldown
    addin
        .set_time_offset(&registrar, &realm_authority, secs_per_day * 94)
        .await;

    // request claim && withdraw
    withdraw(10_000).await.unwrap();

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
            &payer,
            &context.addin.program_id,
        )
        .await;

    let voter_authority = &context.users[1].key;
    let token_owner_record = realm
        .create_token_owner_record(voter_authority.pubkey(), &payer)
        .await;

    let registrar = addin
        .create_registrar(&realm, &realm_authority, payer)
        .await;
    let mngo_voting_mint = addin
        .configure_voting_mint(
            &registrar,
            &realm_authority,
            payer,
            0,
            &context.mints[0],
            0,
            1.0,
            1.0,
            2 * 24 * 60 * 60,
            None,
            None,
        )
        .await;

    let voter = addin
        .create_voter(&registrar, &token_owner_record, &voter_authority, &payer)
        .await;

    let reference_account = context.users[1].token_accounts[0];
    let withdraw = |amount: u64| {
        addin.withdraw(
            &registrar,
            &voter,
            &mngo_voting_mint,
            &voter_authority,
            reference_account,
            0,
            amount,
        )
    };
    let deposit = |amount: u64| {
        addin.deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            &voter_authority,
            reference_account,
            0,
            amount,
        )
    };

    addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter_authority,
            &mngo_voting_mint,
            0,
            LockupKind::Constant,
            None,
            LockupPeriod::ThreeMonths,
        )
        .await
        .unwrap();
    deposit(10000).await.unwrap();

    // advance to 100 days
    addin
        .set_time_offset(&registrar, &realm_authority, 100 * 24 * 60 * 60)
        .await;

    // withdraw
    withdraw(10_000)
        .await
        .expect_err("impossible to withdraw without unlocking");

    Ok(())
}
