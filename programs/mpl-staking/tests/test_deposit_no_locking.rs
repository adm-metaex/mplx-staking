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
    let deposit = voter
        .get_deposit_entry(&context.solana, deposit_id)
        .await
        .amount_deposited_native;
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
async fn test_deposit_no_locking() -> Result<(), TransportError> {
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
    let deposit2_authority = &context.users[2].key;
    let token_owner_record = realm
        .create_token_owner_record(deposit_authority.pubkey(), payer)
        .await;
    let token_owner_record2 = realm
        .create_token_owner_record(deposit2_authority.pubkey(), payer)
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
    let (deposit_mining_voter, _) = find_deposit_mining_addr(
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
            &deposit_mining_voter,
            &context.rewards.program_id,
        )
        .await;

    let voter2_authority = deposit2_authority;
    let (deposit_mining_voter2, _) = find_deposit_mining_addr(
        &context.rewards.program_id,
        &voter2_authority.pubkey(),
        &rewards_pool,
    );
    let voter2 = addin
        .create_voter(
            &registrar,
            &token_owner_record2,
            voter2_authority,
            payer,
            &rewards_pool,
            &deposit_mining_voter2,
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
    let withdraw = |amount: u64| {
        addin.withdraw(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            realm.community_token_account,
            0,
            amount,
        )
    };
    let deposit = |deposit_id: u8, amount: u64| {
        addin.deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            reference_account,
            deposit_id,
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
    deposit(0, 15000).await.unwrap();

    let after_deposit = get_balances(0).await;
    assert_eq!(token, after_deposit.token + after_deposit.vault);
    assert_eq!(after_deposit.voter_weight, after_deposit.vault);
    assert_eq!(after_deposit.vault, 15000);
    assert_eq!(after_deposit.deposit, 15000);

    // create a separate deposit (index 1)
    addin
        .create_deposit_entry(
            &registrar,
            &voter,
            &voter,
            &mngo_voting_mint,
            1,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await
        .unwrap();
    deposit(1, 7000).await.unwrap();

    let after_deposit2 = get_balances(1).await;
    assert_eq!(token, after_deposit2.token + after_deposit2.vault);
    assert_eq!(after_deposit2.voter_weight, after_deposit2.vault);
    assert_eq!(after_deposit2.vault, 22000);
    assert_eq!(after_deposit2.deposit, 7000);

    withdraw(10000).await.unwrap();

    let after_withdraw1 = get_balances(0).await;
    assert_eq!(token, after_withdraw1.token + after_withdraw1.vault);
    assert_eq!(after_withdraw1.voter_weight, after_withdraw1.vault);
    assert_eq!(after_withdraw1.vault, 12000);
    assert_eq!(after_withdraw1.deposit, 5000);

    withdraw(5001)
        .await
        .assert_on_chain_err(MplStakingError::InsufficientUnlockedTokens);

    withdraw(5000).await.unwrap();

    let after_withdraw2 = get_balances(0).await;
    assert_eq!(token, after_withdraw2.token + after_withdraw2.vault);
    assert_eq!(after_withdraw2.voter_weight, after_withdraw2.vault);
    assert_eq!(after_withdraw2.vault, 7000);
    assert_eq!(after_withdraw2.deposit, 0);

    // Close the empty deposit (closing deposits 1 and 2 fails)
    addin
        .close_deposit_entry(&voter, voter_authority, 2)
        .await
        .assert_on_chain_err(MplStakingError::UnusedDepositEntryIndex);
    addin
        .close_deposit_entry(&voter, voter_authority, 1)
        .await
        .assert_on_chain_err(MplStakingError::VotingTokenNonZero);
    addin
        .close_deposit_entry(&voter, voter_authority, 0)
        .await
        .unwrap();

    let after_close = get_balances(0).await;
    assert_eq!(token, after_close.token + after_close.vault);
    assert_eq!(after_close.voter_weight, after_close.vault);
    assert_eq!(after_close.vault, 7000);
    assert_eq!(after_close.deposit, 0);

    // check that the voter2 account is still at 0
    context.solana.advance_clock_by_slots(2).await;
    let voter2_deposit = voter
        .get_deposit_entry(&context.solana, 0)
        .await
        .amount_deposited_native;
    let voter2_voter_weight = context
        .addin
        .update_voter_weight_record(&registrar, &voter2)
        .await
        .unwrap()
        .voter_weight;
    assert_eq!(voter2_deposit, 0);
    assert_eq!(voter2_voter_weight, 0);

    // now voter2 deposits

    addin
        .create_deposit_entry(
            &registrar,
            &voter2,
            &voter2,
            &mngo_voting_mint,
            5,
            LockupKind::None,
            LockupPeriod::None,
        )
        .await
        .unwrap();
    addin
        .deposit(
            &registrar,
            &voter2,
            &mngo_voting_mint,
            voter2_authority,
            context.users[2].token_accounts[0],
            5,
            1000,
        )
        .await
        .unwrap();

    let voter2_balances = balances(
        &context,
        &registrar,
        reference_account,
        &voter2,
        &mngo_voting_mint,
        5,
    )
    .await;
    assert_eq!(voter2_balances.deposit, 1000);
    assert_eq!(voter2_balances.voter_weight, 1000);
    assert_eq!(voter2_balances.vault, 1000);

    // when voter1 deposits again, they can reuse deposit index 0

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
    deposit(0, 3000).await.unwrap();

    let after_reuse = get_balances(0).await;
    assert_eq!(token, after_reuse.token + 7000 + 3000);
    assert_eq!(after_reuse.voter_weight, 7000 + 3000);
    assert_eq!(after_reuse.vault, 7000 + 3000);
    assert_eq!(after_reuse.deposit, 3000);

    Ok(())
}
