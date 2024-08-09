use bytemuck::{bytes_of, Contiguous};
use solana_program::{instruction::InstructionError, program_error::ProgramError};
use solana_program_test::{BanksClientError, ProgramTestContext};
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::{Transaction, TransactionError},
};
use std::borrow::BorrowMut;

#[allow(dead_code)]
pub fn gen_signer_seeds<'a>(nonce: &'a u64, acc_pk: &'a Pubkey) -> [&'a [u8]; 2] {
    [acc_pk.as_ref(), bytes_of(nonce)]
}

#[allow(dead_code)]
pub fn gen_signer_key(
    nonce: u64,
    acc_pk: &Pubkey,
    program_id: &Pubkey,
) -> Result<Pubkey, ProgramError> {
    let seeds = gen_signer_seeds(&nonce, acc_pk);
    Ok(Pubkey::create_program_address(&seeds, program_id)?)
}

#[allow(dead_code)]
pub fn create_signer_key_and_nonce(program_id: &Pubkey, acc_pk: &Pubkey) -> (Pubkey, u64) {
    for i in 0..=u64::MAX_VALUE {
        if let Ok(pk) = gen_signer_key(i, acc_pk, program_id) {
            return (pk, i);
        }
    }
    panic!("Could not generate signer key");
}

#[allow(dead_code)]
pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_base58_string(&keypair.to_base58_string())
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    manager: &Pubkey,
) -> Result<(), BanksClientError> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                manager,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub fn find_deposit_mining_addr(
    program_id: &Pubkey,
    mining_owner: &Pubkey,
    reward_pool: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "mining".as_bytes(),
            &mining_owner.to_bytes(),
            &reward_pool.to_bytes(),
        ],
        program_id,
    )
}

pub async fn advance_clock_by_ts(context: &mut ProgramTestContext, ts: i64) {
    let old_clock = context
        .banks_client
        .get_sysvar::<solana_program::clock::Clock>()
        .await
        .unwrap();

    let initial_slot = context.banks_client.get_root_slot().await.unwrap();
    context
        .warp_to_slot(initial_slot + (ts / 2) as u64)
        .unwrap();

    let mut new_clock = old_clock.clone();
    new_clock.unix_timestamp += ts;
    context.borrow_mut().set_sysvar(&new_clock);
}

pub mod assert_custom_on_chain_error {
    use super::*;
    use mplx_staking_states::error::MplStakingError;
    use std::fmt::Debug;

    pub trait AssertCustomOnChainErr {
        fn assert_on_chain_err(self, expected_err: MplStakingError);
    }

    impl<T: Debug> AssertCustomOnChainErr for Result<T, BanksClientError> {
        fn assert_on_chain_err(self, expected_err: MplStakingError) {
            assert!(self.is_err());
            match self.unwrap_err() {
                BanksClientError::TransactionError(TransactionError::InstructionError(
                    _,
                    InstructionError::Custom(code),
                )) => {
                    debug_assert_eq!((expected_err as u32) + 6000, code);
                }
                _ => unreachable!("BanksClientError has no 'Custom' variant."),
            }
        }
    }
}
