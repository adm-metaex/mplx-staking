use crate::vote_weight_record;
use anchor_lang::prelude::*;
use mplx_staking_states::{error::MplStakingError, state::Registrar};
use spl_governance::state::token_owner_record;

pub fn load_token_owner_record(
    voter_authority: &Pubkey,
    account_info: &AccountInfo,
    registrar: &Registrar,
) -> Result<token_owner_record::TokenOwnerRecordV2> {
    let record = token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint(
        &registrar.governance_program_id,
        account_info,
        &registrar.realm,
        &registrar.realm_governing_token_mint,
    )?;
    require_keys_eq!(
        record.governing_token_owner,
        *voter_authority,
        MplStakingError::InvalidTokenOwnerRecord
    );
    Ok(record)
}

// Generate a VoteWeightRecord Anchor wrapper, owned by the current program.
// VoteWeightRecords are unique in that they are defined by the SPL governance
// program, but they are actually owned by this program.
vote_weight_record!(crate::ID);
