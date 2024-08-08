use anchor_lang::prelude::*;
use static_assertions::const_assert;

/// Exchange rate for an asset that can be used to mint voting rights.
///
/// See documentation of configure_voting_mint for details on how
/// native token amounts convert to vote weight.
#[zero_copy]
#[derive(Default)]
pub struct VotingMintConfig {
    /// Mint for this entry.
    pub mint: Pubkey,

    /// The authority that is allowed to push grants into voters
    pub grant_authority: Pubkey,
}
const_assert!(std::mem::size_of::<VotingMintConfig>() == 2 * 32);
const_assert!(std::mem::size_of::<VotingMintConfig>() % 8 == 0);

impl VotingMintConfig {
    /// Whether this voting mint is configured.
    pub fn in_use(&self) -> bool {
        self.mint != Pubkey::default()
    }
}
