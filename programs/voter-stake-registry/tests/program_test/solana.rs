use anchor_lang::AccountDeserialize;
use anchor_spl::token::TokenAccount;
use solana_program::{program_pack::Pack, rent::*, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::ReadableAccount,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::*;
use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

pub struct SolanaCookie {
    pub context: RefCell<ProgramTestContext>,
    pub rent: Rent,
    pub program_output: Arc<RwLock<super::ProgramOutput>>,
}

impl SolanaCookie {
    #[allow(dead_code)]
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn process_transaction(
        &self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), BanksClientError> {
        *self.program_output.write().unwrap() = super::ProgramOutput::default();

        let mut context = self.context.borrow_mut();

        let mut transaction =
            Transaction::new_with_payer(instructions, Some(&context.payer.pubkey()));

        let mut all_signers = vec![&context.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        // This fails when warping is involved - https://gitmemory.com/issue/solana-labs/solana/18201/868325078
        // let recent_blockhash = self.context.banks_client.get_recent_blockhash().await.unwrap();

        transaction.sign(&all_signers, context.last_blockhash);

        let mut ctx = tarpc::context::Context::current();
        ctx.deadline += std::time::Duration::from_secs(120);

        match context
            .banks_client
            .process_transaction_with_commitment_and_context(
                ctx,
                transaction,
                solana_sdk::commitment_config::CommitmentLevel::Processed,
            )
            .await?
        {
            Some(transaction_result) => Ok(transaction_result?),
            None => Err(BanksClientError::ClientError(
                "invalid blockhash or fee-payer",
            )),
        }
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn get_clock(&self) -> solana_program::clock::Clock {
        self.context
            .borrow_mut()
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub async fn advance_clock_by_slots(&self, slots: u64) {
        let clock = self.get_clock().await;
        self.context
            .borrow_mut()
            .warp_to_slot(clock.slot + slots)
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn create_token_account(&self, owner: &Pubkey, mint: Pubkey) -> Pubkey {
        let keypair = Keypair::new();
        let rent = self.rent.minimum_balance(spl_token::state::Account::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.context.borrow().payer.pubkey(),
                &keypair.pubkey(),
                rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &keypair.pubkey(),
                &mint,
                owner,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&keypair]))
            .await
            .unwrap();
        keypair.pubkey()
    }

    #[allow(dead_code)]
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn get_account_data(&self, address: Pubkey) -> Vec<u8> {
        self.context
            .borrow_mut()
            .banks_client
            .get_account(address)
            .await
            .unwrap()
            .unwrap()
            .data()
            .to_vec()
    }

    pub async fn get_account<T: AccountDeserialize>(&self, address: Pubkey) -> T {
        let data = self.get_account_data(address).await;
        let mut data_slice: &[u8] = &data;
        AccountDeserialize::try_deserialize(&mut data_slice).unwrap()
    }

    #[allow(dead_code)]
    pub async fn token_account_balance(&self, address: Pubkey) -> u64 {
        self.get_account::<TokenAccount>(address).await.amount
    }

    #[allow(dead_code)]
    pub fn program_output(&self) -> super::ProgramOutput {
        self.program_output.read().unwrap().clone()
    }

    #[allow(dead_code)]
    pub async fn create_spl_ata(&self, owner: &Pubkey, mint: &Pubkey, payer: &Keypair) -> Pubkey {
        // let rent = self.rent.minimum_balance(spl_token::state::Account::LEN);

        let (ata_addr, _ata_bump) = Pubkey::find_program_address(
            &[
                &owner.to_bytes(),
                &spl_token::ID.to_bytes(),
                &mint.to_bytes(),
            ],
            &spl_associated_token_account::ID,
        );

        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                owner,
                mint,
                &spl_token::ID,
            );

        let instructions = &[create_ata_ix];

        self.process_transaction(instructions, Some(&[payer]))
            .await
            .unwrap();

        ata_addr
    }
}
