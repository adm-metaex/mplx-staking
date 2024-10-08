export type MplStaking = {
  "version": "0.1.0",
  "name": "mpl_staking",
  "docs": [
    "# Introduction",
    "",
    "The governance registry is an \"addin\" to the SPL governance program that",
    "allows one to both vote with many different ypes of tokens for voting and to",
    "scale voting power as a linear function of time locked--subject to some",
    "maximum upper bound.",
    "",
    "The flow for voting with this program is as follows:",
    "",
    "- Create a SPL governance realm.",
    "- Create a governance registry account.",
    "- Add exchange rates for any tokens one wants to deposit. For example, if one wants to vote with",
    "tokens A and B, where token B has twice the voting power of token A, then the exchange rate of",
    "B would be 2 and the exchange rate of A would be 1.",
    "- Create a voter account.",
    "- Deposit tokens into this program, with an optional lockup period.",
    "- Vote.",
    "",
    "Upon voting with SPL governance, a client is expected to call",
    "`decay_voting_power` to get an up to date measurement of a given `Voter`'s",
    "voting power for the given slot. If this is not done, then the transaction",
    "will fail (since the SPL governance program will require the measurement",
    "to be active for the current slot).",
    "",
    "# Interacting with SPL Governance",
    "",
    "This program does not directly interact with SPL governance via CPI.",
    "Instead, it simply writes a `VoterWeightRecord` account with a well defined",
    "format, which is then used by SPL governance as the voting power measurement",
    "for a given user.",
    "",
    "# Max Vote Weight",
    "",
    "Given that one can use multiple tokens to vote, the max vote weight needs",
    "to be a function of the total supply of all tokens, converted into a common",
    "currency. For example, if you have Token A and Token B, where 1 Token B =",
    "10 Token A, then the `max_vote_weight` should be `supply(A) + supply(B)*10`",
    "where both are converted into common decimals. Then, when calculating the",
    "weight of an individual voter, one can convert B into A via the given",
    "exchange rate, which must be fixed.",
    "",
    "Note that the above also implies that the `max_vote_weight` must fit into",
    "a u64."
  ],
  "instructions": [
    {
      "name": "createRegistrar",
      "accounts": [
        {
          "name": "registrar",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The voting registrar. There can only be a single registrar",
            "per governance realm and governing mint."
          ]
        },
        {
          "name": "realm",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "An spl-governance realm",
            "",
            "- realm is owned by the governance_program_id",
            "- realm_governing_token_mint must be the community or council mint",
            "- realm_authority is realm.authority"
          ]
        },
        {
          "name": "governanceProgramId",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The program id of the spl-governance program the realm belongs to."
          ]
        },
        {
          "name": "realmGoverningTokenMint",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Either the realm community mint or the council mint."
          ]
        },
        {
          "name": "realmAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "rewardVault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "This account is responsible for storing money for rewards",
            "PDA([\"vault\", reward_pool, reward_mint], reward_program)"
          ]
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "registrarBump",
          "type": "u8"
        },
        {
          "name": "fillAuthority",
          "type": "publicKey"
        },
        {
          "name": "distributionAuthority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "configureVotingMint",
      "accounts": [
        {
          "name": "registrar",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "realmAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Tokens of this mint will produce vote weight"
          ]
        }
      ],
      "args": [
        {
          "name": "idx",
          "type": "u16"
        },
        {
          "name": "grantAuthority",
          "type": {
            "option": "publicKey"
          }
        }
      ]
    },
    {
      "name": "createVoter",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Also, Registrar plays the role of deposit_authority on the Rewards Contract,",
            "therefore their PDA that should sign the CPI call"
          ]
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "The authority controling the voter. Must be the same as the",
            "`governing_token_owner` in the token owner record used with",
            "spl-governance."
          ]
        },
        {
          "name": "voterWeightRecord",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The voter weight record is the account that will be shown to spl-governance",
            "to prove how much vote weight the voter has. See update_voter_weight_record."
          ]
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "NOTE: this account is currently unused"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "voterBump",
          "type": "u8"
        },
        {
          "name": "voterWeightRecordBump",
          "type": "u8"
        }
      ]
    },
    {
      "name": "createDepositEntry",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegateVoter",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "depositMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "associatedTokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        },
        {
          "name": "kind",
          "type": {
            "defined": "LockupKind"
          }
        },
        {
          "name": "period",
          "type": {
            "defined": "LockupPeriod"
          }
        }
      ]
    },
    {
      "name": "deposit",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "depositToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "depositAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdraw",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenOwnerRecord",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The token_owner_record for the voter_authority. This is needed",
            "to be able to forbid withdraws while the voter is engaged with",
            "a vote or has an open proposal.",
            "",
            "- owned by registrar.governance_program_id",
            "- for the registrar.realm",
            "- for the registrar.realm_governing_token_mint",
            "- governing_token_owner is voter_authority"
          ]
        },
        {
          "name": "voterWeightRecord",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Withdraws must update the voter weight record, to prevent a stale",
            "record being used to vote after the withdraw."
          ]
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "destination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "closeDepositEntry",
      "accounts": [
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        }
      ]
    },
    {
      "name": "updateVoterWeightRecord",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voterWeightRecord",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "unlockTokens",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegate",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "delegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards progra,",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        }
      ]
    },
    {
      "name": "closeVoter",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "logVoterInfo",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryBegin",
          "type": "u8"
        },
        {
          "name": "depositEntryCount",
          "type": "u8"
        }
      ]
    },
    {
      "name": "stake",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegate",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "delegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards progra,",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "sourceDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "targetDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "extendStake",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegate",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "delegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards progra,",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "sourceDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "targetDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "newLockupPeriod",
          "type": {
            "defined": "LockupPeriod"
          }
        },
        {
          "name": "additionalAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "claim",
      "accounts": [
        {
          "name": "rewardPool",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "rewardMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "is checked on the rewards contract",
            "PDA([\"vault\", reward_pool, reward_mint], reward_program)"
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "miningOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "therefore their PDA that should sign the CPI call"
          ]
        },
        {
          "name": "governance",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Can't be Account<'_, T> because doesn't implement AnchorDeserialize"
          ]
        },
        {
          "name": "proposal",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Can't be Account<'_, T> because doesn't implement AnchorDeserialize"
          ]
        },
        {
          "name": "voteRecord",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Can't be Account<'_, T> because doesn't implement AnchorDeserialize"
          ]
        },
        {
          "name": "userRewardTokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "registrarBump",
          "type": "u8"
        },
        {
          "name": "realmGoverningMintPubkey",
          "type": "publicKey"
        },
        {
          "name": "realmPubkey",
          "type": "publicKey"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "changeDelegate",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegateVoter",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "oldDelegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards program",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)",
            "Seeds derivation will be checked on the rewards contract"
          ]
        },
        {
          "name": "newDelegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards program",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)",
            "Seeds derivation will be checked on the rewards contract"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        }
      ]
    }
  ],
  "types": [
    {
      "name": "VestingInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "rate",
            "docs": [
              "Amount of tokens vested each period"
            ],
            "type": "u64"
          },
          {
            "name": "nextTimestamp",
            "docs": [
              "Time of the next upcoming vesting"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LockingInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amount",
            "docs": [
              "Amount of locked tokens"
            ],
            "type": "u64"
          },
          {
            "name": "endTimestamp",
            "docs": [
              "Time at which the lockup fully ends (None for Constant lockup)"
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "vesting",
            "docs": [
              "Information about vesting, if any"
            ],
            "type": {
              "option": {
                "defined": "VestingInfo"
              }
            }
          }
        ]
      }
    },
    {
      "name": "RewardsInstruction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "InitializePool",
            "fields": [
              {
                "name": "fill_authority",
                "docs": [
                  "Account can fill the reward vault"
                ],
                "type": "publicKey"
              },
              {
                "name": "distribution_authority",
                "docs": [
                  "Account can distribute rewards for stakers"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "FillVault",
            "fields": [
              {
                "name": "amount",
                "docs": [
                  "Amount to fill"
                ],
                "type": "u64"
              },
              {
                "name": "distribution_ends_at",
                "docs": [
                  "Rewards distribution ends at given date"
                ],
                "type": "u64"
              }
            ]
          },
          {
            "name": "InitializeMining",
            "fields": [
              {
                "name": "mining_owner",
                "docs": [
                  "Represent the end-user, owner of the mining"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "DepositMining",
            "fields": [
              {
                "name": "amount",
                "docs": [
                  "Amount to deposit"
                ],
                "type": "u64"
              },
              {
                "name": "lockup_period",
                "docs": [
                  "Lockup Period"
                ],
                "type": {
                  "defined": "LockupPeriod"
                }
              },
              {
                "name": "owner",
                "docs": [
                  "Specifies the owner of the Mining Account"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "WithdrawMining",
            "fields": [
              {
                "name": "amount",
                "docs": [
                  "Amount to withdraw"
                ],
                "type": "u64"
              },
              {
                "name": "owner",
                "docs": [
                  "Specifies the owner of the Mining Account"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "Claim"
          },
          {
            "name": "ExtendStake",
            "fields": [
              {
                "name": "old_lockup_period",
                "docs": [
                  "Lockup period before restaking. Actually it's only needed",
                  "for Flex to AnyPeriod edge case"
                ],
                "type": {
                  "defined": "LockupPeriod"
                }
              },
              {
                "name": "new_lockup_period",
                "docs": [
                  "Requested lockup period for restaking"
                ],
                "type": {
                  "defined": "LockupPeriod"
                }
              },
              {
                "name": "deposit_start_ts",
                "docs": [
                  "Deposit start_ts"
                ],
                "type": "u64"
              },
              {
                "name": "base_amount",
                "docs": [
                  "Amount of tokens to be restaked, this",
                  "number cannot be decreased. It reflects the number of staked tokens",
                  "before the extend_stake function call"
                ],
                "type": "u64"
              },
              {
                "name": "additional_amount",
                "docs": [
                  "In case user wants to increase it's staked number of tokens,",
                  "the addition amount might be provided"
                ],
                "type": "u64"
              },
              {
                "name": "mining_owner",
                "docs": [
                  "The wallet who owns the mining account"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "DistributeRewards"
          },
          {
            "name": "CloseMining"
          },
          {
            "name": "ChangeDelegate",
            "fields": [
              {
                "name": "staked_amount",
                "docs": [
                  "Amount of staked tokens"
                ],
                "type": "u64"
              }
            ]
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "VoterInfo",
      "fields": [
        {
          "name": "votingPower",
          "type": "u64",
          "index": false
        },
        {
          "name": "votingPowerBaseline",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "DepositEntryInfo",
      "fields": [
        {
          "name": "depositEntryIndex",
          "type": "u8",
          "index": false
        },
        {
          "name": "votingMintConfigIndex",
          "type": "u8",
          "index": false
        },
        {
          "name": "unlocked",
          "type": "u64",
          "index": false
        },
        {
          "name": "votingPower",
          "type": "u64",
          "index": false
        },
        {
          "name": "votingPowerBaseline",
          "type": "u64",
          "index": false
        },
        {
          "name": "locking",
          "type": {
            "option": {
              "defined": "LockingInfo"
            }
          },
          "index": false
        }
      ]
    }
  ]
};

export const IDL: MplStaking = {
  "version": "0.1.0",
  "name": "mpl_staking",
  "docs": [
    "# Introduction",
    "",
    "The governance registry is an \"addin\" to the SPL governance program that",
    "allows one to both vote with many different ypes of tokens for voting and to",
    "scale voting power as a linear function of time locked--subject to some",
    "maximum upper bound.",
    "",
    "The flow for voting with this program is as follows:",
    "",
    "- Create a SPL governance realm.",
    "- Create a governance registry account.",
    "- Add exchange rates for any tokens one wants to deposit. For example, if one wants to vote with",
    "tokens A and B, where token B has twice the voting power of token A, then the exchange rate of",
    "B would be 2 and the exchange rate of A would be 1.",
    "- Create a voter account.",
    "- Deposit tokens into this program, with an optional lockup period.",
    "- Vote.",
    "",
    "Upon voting with SPL governance, a client is expected to call",
    "`decay_voting_power` to get an up to date measurement of a given `Voter`'s",
    "voting power for the given slot. If this is not done, then the transaction",
    "will fail (since the SPL governance program will require the measurement",
    "to be active for the current slot).",
    "",
    "# Interacting with SPL Governance",
    "",
    "This program does not directly interact with SPL governance via CPI.",
    "Instead, it simply writes a `VoterWeightRecord` account with a well defined",
    "format, which is then used by SPL governance as the voting power measurement",
    "for a given user.",
    "",
    "# Max Vote Weight",
    "",
    "Given that one can use multiple tokens to vote, the max vote weight needs",
    "to be a function of the total supply of all tokens, converted into a common",
    "currency. For example, if you have Token A and Token B, where 1 Token B =",
    "10 Token A, then the `max_vote_weight` should be `supply(A) + supply(B)*10`",
    "where both are converted into common decimals. Then, when calculating the",
    "weight of an individual voter, one can convert B into A via the given",
    "exchange rate, which must be fixed.",
    "",
    "Note that the above also implies that the `max_vote_weight` must fit into",
    "a u64."
  ],
  "instructions": [
    {
      "name": "createRegistrar",
      "accounts": [
        {
          "name": "registrar",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The voting registrar. There can only be a single registrar",
            "per governance realm and governing mint."
          ]
        },
        {
          "name": "realm",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "An spl-governance realm",
            "",
            "- realm is owned by the governance_program_id",
            "- realm_governing_token_mint must be the community or council mint",
            "- realm_authority is realm.authority"
          ]
        },
        {
          "name": "governanceProgramId",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The program id of the spl-governance program the realm belongs to."
          ]
        },
        {
          "name": "realmGoverningTokenMint",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Either the realm community mint or the council mint."
          ]
        },
        {
          "name": "realmAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "rewardVault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "This account is responsible for storing money for rewards",
            "PDA([\"vault\", reward_pool, reward_mint], reward_program)"
          ]
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "registrarBump",
          "type": "u8"
        },
        {
          "name": "fillAuthority",
          "type": "publicKey"
        },
        {
          "name": "distributionAuthority",
          "type": "publicKey"
        }
      ]
    },
    {
      "name": "configureVotingMint",
      "accounts": [
        {
          "name": "registrar",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "realmAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Tokens of this mint will produce vote weight"
          ]
        }
      ],
      "args": [
        {
          "name": "idx",
          "type": "u16"
        },
        {
          "name": "grantAuthority",
          "type": {
            "option": "publicKey"
          }
        }
      ]
    },
    {
      "name": "createVoter",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Also, Registrar plays the role of deposit_authority on the Rewards Contract,",
            "therefore their PDA that should sign the CPI call"
          ]
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "The authority controling the voter. Must be the same as the",
            "`governing_token_owner` in the token owner record used with",
            "spl-governance."
          ]
        },
        {
          "name": "voterWeightRecord",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The voter weight record is the account that will be shown to spl-governance",
            "to prove how much vote weight the voter has. See update_voter_weight_record."
          ]
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rent",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "instructions",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "NOTE: this account is currently unused"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "voterBump",
          "type": "u8"
        },
        {
          "name": "voterWeightRecordBump",
          "type": "u8"
        }
      ]
    },
    {
      "name": "createDepositEntry",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegateVoter",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "payer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "depositMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "associatedTokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        },
        {
          "name": "kind",
          "type": {
            "defined": "LockupKind"
          }
        },
        {
          "name": "period",
          "type": {
            "defined": "LockupPeriod"
          }
        }
      ]
    },
    {
      "name": "deposit",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "depositToken",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "depositAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdraw",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "tokenOwnerRecord",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The token_owner_record for the voter_authority. This is needed",
            "to be able to forbid withdraws while the voter is engaged with",
            "a vote or has an open proposal.",
            "",
            "- owned by registrar.governance_program_id",
            "- for the registrar.realm",
            "- for the registrar.realm_governing_token_mint",
            "- governing_token_owner is voter_authority"
          ]
        },
        {
          "name": "voterWeightRecord",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Withdraws must update the voter weight record, to prevent a stale",
            "record being used to vote after the withdraw."
          ]
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "destination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "closeDepositEntry",
      "accounts": [
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        }
      ]
    },
    {
      "name": "updateVoterWeightRecord",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voterWeightRecord",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "unlockTokens",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegate",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "delegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards progra,",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        }
      ]
    },
    {
      "name": "closeVoter",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "solDestination",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": []
    },
    {
      "name": "logVoterInfo",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryBegin",
          "type": "u8"
        },
        {
          "name": "depositEntryCount",
          "type": "u8"
        }
      ]
    },
    {
      "name": "stake",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegate",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "delegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards progra,",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "sourceDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "targetDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "extendStake",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegate",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "delegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards progra,",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "sourceDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "targetDepositEntryIndex",
          "type": "u8"
        },
        {
          "name": "newLockupPeriod",
          "type": {
            "defined": "LockupPeriod"
          }
        },
        {
          "name": "additionalAmount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "claim",
      "accounts": [
        {
          "name": "rewardPool",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "rewardMint",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "vault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "is checked on the rewards contract",
            "PDA([\"vault\", reward_pool, reward_mint], reward_program)"
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "miningOwner",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "therefore their PDA that should sign the CPI call"
          ]
        },
        {
          "name": "governance",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Can't be Account<'_, T> because doesn't implement AnchorDeserialize"
          ]
        },
        {
          "name": "proposal",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Can't be Account<'_, T> because doesn't implement AnchorDeserialize"
          ]
        },
        {
          "name": "voteRecord",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Can't be Account<'_, T> because doesn't implement AnchorDeserialize"
          ]
        },
        {
          "name": "userRewardTokenAccount",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "registrarBump",
          "type": "u8"
        },
        {
          "name": "realmGoverningMintPubkey",
          "type": "publicKey"
        },
        {
          "name": "realmPubkey",
          "type": "publicKey"
        }
      ],
      "returns": "u64"
    },
    {
      "name": "changeDelegate",
      "accounts": [
        {
          "name": "registrar",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "voter",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "voterAuthority",
          "isMut": false,
          "isSigner": true
        },
        {
          "name": "delegateVoter",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "oldDelegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards program",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)",
            "Seeds derivation will be checked on the rewards contract"
          ]
        },
        {
          "name": "newDelegateMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The address of the mining account on the rewards program",
            "derived from PDA([\"mining\", delegate wallet addr, reward_pool], rewards_program)",
            "Seeds derivation will be checked on the rewards contract"
          ]
        },
        {
          "name": "rewardPool",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Ownership of the account will be checked in the rewards contract",
            "It's the core account for the rewards contract, which will",
            "keep track of all rewards and staking logic."
          ]
        },
        {
          "name": "depositMining",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "PDA([\"mining\", mining owner <aka voter_authority in our case>, reward_pool],",
            "reward_program)"
          ]
        },
        {
          "name": "rewardsProgram",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "depositEntryIndex",
          "type": "u8"
        }
      ]
    }
  ],
  "types": [
    {
      "name": "VestingInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "rate",
            "docs": [
              "Amount of tokens vested each period"
            ],
            "type": "u64"
          },
          {
            "name": "nextTimestamp",
            "docs": [
              "Time of the next upcoming vesting"
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "LockingInfo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amount",
            "docs": [
              "Amount of locked tokens"
            ],
            "type": "u64"
          },
          {
            "name": "endTimestamp",
            "docs": [
              "Time at which the lockup fully ends (None for Constant lockup)"
            ],
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "vesting",
            "docs": [
              "Information about vesting, if any"
            ],
            "type": {
              "option": {
                "defined": "VestingInfo"
              }
            }
          }
        ]
      }
    },
    {
      "name": "RewardsInstruction",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "InitializePool",
            "fields": [
              {
                "name": "fill_authority",
                "docs": [
                  "Account can fill the reward vault"
                ],
                "type": "publicKey"
              },
              {
                "name": "distribution_authority",
                "docs": [
                  "Account can distribute rewards for stakers"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "FillVault",
            "fields": [
              {
                "name": "amount",
                "docs": [
                  "Amount to fill"
                ],
                "type": "u64"
              },
              {
                "name": "distribution_ends_at",
                "docs": [
                  "Rewards distribution ends at given date"
                ],
                "type": "u64"
              }
            ]
          },
          {
            "name": "InitializeMining",
            "fields": [
              {
                "name": "mining_owner",
                "docs": [
                  "Represent the end-user, owner of the mining"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "DepositMining",
            "fields": [
              {
                "name": "amount",
                "docs": [
                  "Amount to deposit"
                ],
                "type": "u64"
              },
              {
                "name": "lockup_period",
                "docs": [
                  "Lockup Period"
                ],
                "type": {
                  "defined": "LockupPeriod"
                }
              },
              {
                "name": "owner",
                "docs": [
                  "Specifies the owner of the Mining Account"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "WithdrawMining",
            "fields": [
              {
                "name": "amount",
                "docs": [
                  "Amount to withdraw"
                ],
                "type": "u64"
              },
              {
                "name": "owner",
                "docs": [
                  "Specifies the owner of the Mining Account"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "Claim"
          },
          {
            "name": "ExtendStake",
            "fields": [
              {
                "name": "old_lockup_period",
                "docs": [
                  "Lockup period before restaking. Actually it's only needed",
                  "for Flex to AnyPeriod edge case"
                ],
                "type": {
                  "defined": "LockupPeriod"
                }
              },
              {
                "name": "new_lockup_period",
                "docs": [
                  "Requested lockup period for restaking"
                ],
                "type": {
                  "defined": "LockupPeriod"
                }
              },
              {
                "name": "deposit_start_ts",
                "docs": [
                  "Deposit start_ts"
                ],
                "type": "u64"
              },
              {
                "name": "base_amount",
                "docs": [
                  "Amount of tokens to be restaked, this",
                  "number cannot be decreased. It reflects the number of staked tokens",
                  "before the extend_stake function call"
                ],
                "type": "u64"
              },
              {
                "name": "additional_amount",
                "docs": [
                  "In case user wants to increase it's staked number of tokens,",
                  "the addition amount might be provided"
                ],
                "type": "u64"
              },
              {
                "name": "mining_owner",
                "docs": [
                  "The wallet who owns the mining account"
                ],
                "type": "publicKey"
              }
            ]
          },
          {
            "name": "DistributeRewards"
          },
          {
            "name": "CloseMining"
          },
          {
            "name": "ChangeDelegate",
            "fields": [
              {
                "name": "staked_amount",
                "docs": [
                  "Amount of staked tokens"
                ],
                "type": "u64"
              }
            ]
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "VoterInfo",
      "fields": [
        {
          "name": "votingPower",
          "type": "u64",
          "index": false
        },
        {
          "name": "votingPowerBaseline",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "DepositEntryInfo",
      "fields": [
        {
          "name": "depositEntryIndex",
          "type": "u8",
          "index": false
        },
        {
          "name": "votingMintConfigIndex",
          "type": "u8",
          "index": false
        },
        {
          "name": "unlocked",
          "type": "u64",
          "index": false
        },
        {
          "name": "votingPower",
          "type": "u64",
          "index": false
        },
        {
          "name": "votingPowerBaseline",
          "type": "u64",
          "index": false
        },
        {
          "name": "locking",
          "type": {
            "option": {
              "defined": "LockingInfo"
            }
          },
          "index": false
        }
      ]
    }
  ]
};
