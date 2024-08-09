# Description

MPL Staking is a deeply-modified fork of [voter-stake-registry](https://github.com/blockworks-foundation/voter-stake-registry) which is a voter weight addin for Solana's.
[spl-governance program](https://github.com/solana-labs/solana-program-library/tree/master/governance).
MPL Staking is highly dependent on the [MPL Rewards]() program, which is built for rewards calculations that are based on the weighted stake model with arbitrary (and configurable) modifiers.

The essential motivation for this addin is to provide users with the ability of staking and receiving rewards, using the DAO. Batch-minting that is provided by [Bubblegum]() is relied on that program as well.

Having the addin enabled, the realm governance receiving the possibility of:
- Controlling which token mints can be used to vote, however scaling factor remains 1:1 that stands for deposited_tokens:voting_power.
- Clawback operations (TBD)
- Grant operations (TBD)
- Slashing providers for misbehavior (TBD)

Users can:

- Deposit and withdraw tokens.
- Lock up tokens for arbitrary ranges of periods, which gives the opportunity for being eligible for rewards when they occur. Voting power remains the same (1:1) as it was mentioned previously.

  When an addin is enabled, the default deposit/withdraw flow of the governing
  token mints is disabled in spl-governance. The addin adds back the ability
  to deposit and withdraw without lockup.
- The tokens will only be withdrawable when the lockup period has expired, and the cooldown time has passed. Also, user can't withdraw tokens if they have votes in proposals that hasn't been ended yet.
- Use their deposited tokens for voting on spl-governance proposals.

# Development

## Rust
* Built and developed using - `rustc 1.66.1` and `anchor 1.14.10`
* Run rust based tests - `cargo test-spf`
* Run `cargo +nightly fmt` before pushing your changes

# Deployment

Users may want to compile their own mpl-staking and deploy it to an address they control.

Before compiling, look at:
- `Registrar::voting_mints`: The length of this array defines the number of configurable voting mints. Adjust as needed.
- `const DAO_PUBKEY`: The address of this constant must be specified before the contract deployment. It specifies the address of a DAO an end user should interact with to claim rewards.


# Usage Scenarios

## Setup

To start using the addin, make a governance proposal with the spl-governance
realm authority to:
1. Deploy an instance of the mpl-staking.
2. Create a registrar for the realm with the `CreateRegistrar` instruction.
3. Add voting token mints to the registrar by calling the `ConfigureVotingMint`
   instruction as often as desired.
4. Call the `SetRealmConfig` instruction on spl-governance to set the
   voter-weight-addin program id and thereby enable the addin.

## Deposit and Vote Without Lockup

Interaction is intended to be user friendly, so an end-user probably want a dedicated UI to interact with DAO.
Nevertheless, it might be done manually by invoking transactions directly.

1. Call `CreateVoter` on the addin (first time only). Use the same
   `voter_authority` that was used for registering with `spl-governance`. This action is necessary to be able to interact with the program.
2. Call `CreateDepositEntry` for the voter with `LockupKind::None`
   and the token mint for that tokens are to be deposited (first time only).
   `LockupKind::None` means that your tokens aren't locked and can be freely withdrawn or deposited again.
3. Call `Deposit` to transfer tokens from your wallet into the freshly created `DepositEnty`. Now you can interact with the DAO, including voting, etc.
4. To vote, call `UpdateVoterWeightRecord` on the addin and then call `CastVote`
    on spl-governance in the same transaction, passing the voter weight record
    to both.
5. If the end user want to stake, then they should create one more deposit entry with `LockupKind::Constant` and  required parameters, like `lockup_period`, `amount`. Then  call `Stake`, providing "internal" indexes for the both of `DepositEntry`s.

6. Withdraw funds with `Withdraw` once proposals have resolved + unstake operation has been requested and cooldown has expired in case of staked tokens.

# Instruction Overview

## Setup

- [`CreateRegistrar`](programs/mpl-rewards/src/instructions/create_registrar.rs)

  Creates a Registrar account for a governance realm.

- [`ConfigureVotingMint`](programs/mpl-rewards/src/instructions/configure_voting_mint.rs)

  Enables voting with tokens from a mint and sets the set of authorities (grant/clawback/etc).

## Usage

- [`CreateVoter`](programs/mpl-rewards/src/instructions/create_voter.rs)

  Create a new voter account for a user. Additionally, this function creates `Mining` account as a part of the Rewards contract. That account will store all rewards that might be payed to the user.

- [`CreateDepositEntry`](programs/mpl-rewards/src/instructions/create_deposit_entry.rs)

  Create a deposit entry on a voter with. A deposit entry is where tokens from a voting mint
  are deposited, and which may optionally have a lockup period (configurable via parameters that pass in).

  Each voter can have multiple deposit entries (up to 32).

- [`Claim`](programs/mpl-staking/src/instructions/claim.rs)
  If some rewards distribution happened and an end-user had tokens staked at the moment, they receive rewards. In case there are any rewards, the usage of this instruction will transfer them to the specified address.

- [`CloseDepositEntry`](programs/mpl-staking/src/instructions/close_deposit_entry.rs)

  Close an empty deposit entry, so it can be reused for a different mint or lockup type. Deposit entries can only be closed when they don't hold any tokens.

- [`CloseVoter`](programs/mpl-staking/src/instructions/close_voter.rs)

  Close an empty voter, reclaiming rent. CPI will be made to the rewards contract to close the mining account and reclaim rent for that account as well.

- [`ChangeDelegate`](programs/mpl-staking/src/instructions/change_delegate.rs)
  User has an opportunity to stake their token through the chosen delegate. This gives a possibility to use batch minting, albeit potential rewards will be slightly reduced. (It depends on the number of Mining accounts in the pool, their weighted stakes etc.)
  Additionally, CPI will be made to recalculated weighted staked of the delegates.

- [`Deposit`](programs/mpl-rewards/src/instructions/deposit.rs)

  Add tokens to a `DepositEntry` if the `DepositEntry` is not locked.

- [`ExtendStake`](programs/mpl-staking/src/instructions/extend_stake.rs)
  User may want to prolong their stakes or to stake additional money. It might be hard to achieve with the limit of 32 stakes, so each of stakes might be extended. Please, take note that stake may only be prolonged for the longer period (e.g. `ThreeMonths` to `OneYear`, the reverse operation is prohibited) and increasing staked tokens only allowed for such operation. It's impossible to lock more tokens without changing `LockupPeriod`.

- [`Stake`](programs/mpl-staking/src/instructions/extend_stake.rs)
  This operation locks the stake up for specified amount of time specified in during creation of the `DepositEntry`. Instruction also does a CPI to the rewards program to write down increased number of stake which leads to the rewards increasing.

- [`UpdateVoterWeightRecord`](programs/mpl-staking/src/instructions/update_voter_weight_record.rs)

  Write the current voter weight to the account that spl-governance can read to
  prepare for voting.

- [`UnlockTokens`](programs/mpl-staking/src/instructions/unlock_tokens.rs)
  Makes a request for a deposit unlocking. It means, the call well be registered and after the cooldown period has expired, tokens are ready to be withdrawn. `UnlockTokens` operation is available immediately, though cooldown have to pass first the user is allowed to withdraw their tokens.
  Also, this operation does a CPI to the rewards contract. That means, when used, user no longer will be accounted as a part of rewards distribution (for one selected stake, not in general).

- [`Withdraw`](programs/mpl-staking/src/instructions/withdraw.rs)

  Remove tokens from a deposit entry of any kind. The operation will be successful only if required conditions are met.

## Special

- [`Grant`](programs/mpl-staking/src/instructions/grant.rs) TBD

- [`Clawback`](programs/mpl-staking/src/instructions/clawback.rs) TBD

- [`Slash`](programs/mpl-staking/src/instructions/slash.rs) TBD


# License

This code is currently not free to use while in development.


# References:
* [spl-governance](https://github.com/solana-labs/solana-program-library/tree/master/governance)
