use anchor_spl::token::TokenAccount;
use mpl_staking::state::{LockupKind, LockupPeriod};
use program_test::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};

mod program_test;

fn deserialize_event<T: anchor_lang::Event>(event: &str) -> Option<T> {
    let data = base64::decode(event).ok()?;
    if data.len() < 8 || data[0..8] != T::discriminator() {
        return None;
    }
    T::try_from_slice(&data[8..]).ok()
}

#[tokio::test]
async fn test_print_event() -> Result<(), TransportError> {
    println!(
        "{:#?}",
        deserialize_event::<mpl_staking::events::DepositEntryInfo>(
            "LP4gbyknBZQAABhzAQAAAAAAGHMBAAAAAAAYcwEAAAAAAAEAAAAAAAAAAAGK6hx3fgEAAAA="
        )
        .ok_or(())
    );
    Ok(())
}

#[tokio::test]
async fn test_log_voter_info() -> Result<(), TransportError> {
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
    let voter_mngo = context.users[1].token_accounts[0];
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
            LockupPeriod::OneYear,
        )
        .await
        .unwrap();
    addin
        .deposit(
            &registrar,
            &voter,
            &mngo_voting_mint,
            voter_authority,
            voter_mngo,
            0,
            12000,
        )
        .await
        .unwrap();
    addin
        .stake(
            &registrar,
            &voter,
            voter.authority.pubkey(),
            &context.rewards.program_id,
            0,
            1,
            12000,
        )
        .await?;

    // advance time to one month ahead
    addin
        .set_time_offset(&registrar, &realm_authority, 365 * 24 * 60 * 60 / 12)
        .await;
    context.solana.advance_clock_by_slots(2).await;

    addin.log_voter_info(&registrar, &voter, 0).await;
    let data_log = context.solana.program_output().data;
    assert_eq!(data_log.len(), 3);

    let voter_event = deserialize_event::<mpl_staking::events::VoterInfo>(&data_log[0]).unwrap();
    assert_eq!(voter_event.voting_power_baseline, 12000);
    assert_eq!(voter_event.voting_power, 12000);

    let deposit_event =
        deserialize_event::<mpl_staking::events::DepositEntryInfo>(&data_log[1]).unwrap();
    assert_eq!(deposit_event.deposit_entry_index, 0);
    assert_eq!(deposit_event.voting_mint_config_index, 0);
    assert_eq!(deposit_event.unlocked, 0);

    let deposit_event =
        deserialize_event::<mpl_staking::events::DepositEntryInfo>(&data_log[2]).unwrap();
    assert_eq!(deposit_event.voting_power, voter_event.voting_power);
    assert_eq!(
        deposit_event.voting_power_baseline,
        voter_event.voting_power_baseline
    );
    assert!(deposit_event.locking.is_some());
    let locking = deposit_event.locking.unwrap();
    assert!(locking.vesting.is_none());
    assert_eq!(locking.amount, 12000);

    Ok(())
}
