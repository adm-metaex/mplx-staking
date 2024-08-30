pub use allow_tokenflow::*;
use anchor_lang::{
    prelude::{AccountLoader, Signer, ToAccountInfo, UncheckedAccount},
    Accounts,
};
pub use restrict_batch_minting::*;
pub use restrict_tokenflow::*;

mod allow_tokenflow;
mod restrict_batch_minting;
mod restrict_tokenflow;
pub use slash::*;
mod slash;

use mplx_staking_states::state::Registrar;

#[derive(Accounts)]
pub struct Penalty<'info> {
    pub registrar: AccountLoader<'info, Registrar>,

    pub realm_authority: Signer<'info>,

    /// CHECK:
    /// Ownership of the account will be checked in the rewards contract
    /// It's the core account for the rewards contract, which will
    /// keep track of all rewards and staking logic.
    pub reward_pool: UncheckedAccount<'info>,

    /// CHECK: mining PDA will be checked in the rewards contract
    /// PDA(["mining", mining owner <aka voter_authority in our case>, reward_pool],
    /// reward_program)
    #[account(mut)]
    pub deposit_mining: UncheckedAccount<'info>,

    /// CHECK: Rewards Program account
    #[account(executable)]
    pub rewards_program: UncheckedAccount<'info>,
}
