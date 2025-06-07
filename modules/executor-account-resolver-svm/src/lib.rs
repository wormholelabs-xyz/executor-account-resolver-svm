#![feature(try_trait_v2)]
use std::ops::{FromResidual, Try};

use anchor_lang::{prelude::*, solana_program::instruction::Instruction, Bumps};

// hash inputs
/// The hash input for `RESOLVER_EXECUTE_VAA_V1`.
pub const RESOLVER_EXECUTE_VAA_V1_SEED: &[u8] = b"executor-account-resolver:execute-vaa-v1";
/// The PDA seed for calculating the return account.
pub const RESOLVER_RESULT_ACCOUNT_SEED: &[u8] = b"executor-account-resolver:result";

// discriminators
/// Discriminator for resolving the instructions for executing a v1 VAA.
///
/// Usage:
///
/// ```rust
/// #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
/// pub fn accounts_to_execute(ctx: Context<Resolve>) -> Result<Resolver<InstructionGroups>> {
///     Ok(Resolver::Resolved(InstructionGroups(vec![...])))
/// }
/// ```
///
/// Ensure that you have the `interface-instructions` feature enabled.
pub const RESOLVER_EXECUTE_VAA_V1: [u8; 8] = [148, 184, 169, 222, 207, 8, 154, 127];

// account placeholders
// these follow the padding pattern of https://github.com/wormhole-foundation/wormhole/blob/main/whitepapers/0009_guardian_signer.md#prefixes-used
/// A placeholder to represent the relayer's account that will pay for the transaction.
/// This will be replaced by the relayer with their pubkey.
pub const RESOLVER_PUBKEY_PAYER: Pubkey =
    Pubkey::new_from_array(*b"payer_00000000000000000000000000");
/// A placeholder to represent the Wormhole Core Bridge Posted VAA.
/// This will be replaced by the relayer with the posted VAA to be executed.
pub const RESOLVER_PUBKEY_POSTED_VAA: Pubkey =
    Pubkey::new_from_array(*b"posted_vaa_000000000000000000000");
/// A placeholder to represent the Wormhole Verify VAA Shim Guardian Signatures account.
/// This will be replaced by the relayer with an account with the signatures of the VAA to be executed.
/// See https://github.com/wormhole-foundation/wormhole/tree/fe4a33bafae3eb2ba51dff16efaab70e50be111d/svm/wormhole-core-shims/programs/verify-vaa for more info.
pub const RESOLVER_PUBKEY_SHIM_VAA_SIGS: Pubkey =
    Pubkey::new_from_array(*b"shim_vaa_sigs_000000000000000000");
/// A placeholder to represent a new keypair's pubkey.
/// This will be replaced by the relayer with a newly generated keypair.
/// This can be used to make a new account in one instruction and refer to the same account in subsequent instructions.
/// Consts for 10 accounts are provided.
pub const RESOLVER_PUBKEY_KEYPAIR_00: Pubkey =
    Pubkey::new_from_array(*b"keypair_00_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_01: Pubkey =
    Pubkey::new_from_array(*b"keypair_01_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_02: Pubkey =
    Pubkey::new_from_array(*b"keypair_02_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_03: Pubkey =
    Pubkey::new_from_array(*b"keypair_03_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_04: Pubkey =
    Pubkey::new_from_array(*b"keypair_04_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_05: Pubkey =
    Pubkey::new_from_array(*b"keypair_05_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_06: Pubkey =
    Pubkey::new_from_array(*b"keypair_06_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_07: Pubkey =
    Pubkey::new_from_array(*b"keypair_07_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_08: Pubkey =
    Pubkey::new_from_array(*b"keypair_08_000000000000000000000");
pub const RESOLVER_PUBKEY_KEYPAIR_09: Pubkey =
    Pubkey::new_from_array(*b"keypair_09_000000000000000000000");

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InstructionGroups(pub Vec<InstructionGroup>);

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InstructionGroup {
    pub instructions: Vec<SerializableInstruction>,
    pub address_lookup_tables: Vec<Pubkey>,
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SerializableInstruction {
    pub program_id: Pubkey,
    pub accounts: Vec<SerializableAccountMeta>,
    pub data: Vec<u8>,
}

impl From<Instruction> for SerializableInstruction {
    fn from(instruction: Instruction) -> Self {
        SerializableInstruction {
            program_id: instruction.program_id,
            accounts: instruction
                .accounts
                .into_iter()
                .map(|account_meta| account_meta.into())
                .collect(),
            data: instruction.data,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SerializableAccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl From<AccountMeta> for SerializableAccountMeta {
    fn from(account_meta: AccountMeta) -> Self {
        SerializableAccountMeta {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}

pub struct RemainingAccounts<'c, 'info> {
    pub remaining_accounts: &'c [AccountInfo<'info>],
}

impl<'c, 'info> RemainingAccounts<'c, 'info> {
    pub fn load<'a>(&'a self, key: Pubkey) -> Resolver<&'c AccountInfo<'info>> {
        load(self.remaining_accounts, key)
    }

    pub fn load_deserialize<'a, T: AccountDeserialize>(&'a self, key: Pubkey) -> Resolver<T> {
        load_deserialize(self.remaining_accounts, key)
    }
}

impl<'a, 'b, 'c, 'info, T: Bumps> From<Context<'a, 'b, 'c, 'info, T>>
    for RemainingAccounts<'c, 'info>
{
    fn from(ctx: Context<'a, 'b, 'c, 'info, T>) -> Self {
        RemainingAccounts {
            remaining_accounts: ctx.remaining_accounts,
        }
    }
}

fn load<'c, 'info>(
    accs: &'c [AccountInfo<'info>],
    key: Pubkey,
) -> Resolver<&'c AccountInfo<'info>> {
    if let Some(found) = accs.iter().find(|acc_info| *acc_info.key == key) {
        Resolver::Resolved(found)
    } else {
        Resolver::Missing(MissingAccounts {
            accounts: vec![key],
            address_lookup_tables: vec![],
        })
    }
}

fn load_deserialize<'c, 'info, T: AccountDeserialize>(
    accs: &'c [AccountInfo<'info>],
    key: Pubkey,
) -> Resolver<T> {
    let acc_info = load(accs, key)?;
    let data = T::try_deserialize(&mut &acc_info.data.borrow()[..]).unwrap();
    Resolver::Resolved(data)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum Resolver<T> {
    Resolved(T),
    Missing(MissingAccounts),
    Account(),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MissingAccounts {
    pub accounts: Vec<Pubkey>,
    pub address_lookup_tables: Vec<Pubkey>,
}

// impl<T> Resolver<T> {
//     pub fn pair<U>(self, other: Resolver<U>) -> Resolver<(T, U)> {
//         pair(self, other)
//     }
// }

// pub fn pair<T, U>(a: Resolver<T>, b: Resolver<U>) -> Resolver<(T, U)> {
//     match (a, b) {
//         (Resolver::Resolved(a), Resolver::Resolved(b)) => Resolver::Resolved((a, b)),
//         (Resolver::Resolved(_), Resolver::Missing(missing)) => Resolver::Missing(missing),
//         (Resolver::Missing(missing), Resolver::Resolved(_)) => Resolver::Missing(missing),
//         (Resolver::Missing(mut missing_a), Resolver::Missing(missing_b)) => {
//             missing_a.accounts.extend(missing_b.accounts);
//             missing_a
//                 .address_lookup_tables
//                 .extend(missing_b.address_lookup_tables);
//             Resolver::Missing(missing_a)
//         }
//     }
// }

impl<T> FromResidual for Resolver<T> {
    fn from_residual(residual: MissingAccounts) -> Self {
        Resolver::Missing(residual)
    }
}

impl<T> Try for Resolver<T> {
    type Output = T;

    type Residual = MissingAccounts;

    fn from_output(output: Self::Output) -> Self {
        Resolver::Resolved(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Resolver::Resolved(output) => std::ops::ControlFlow::Continue(output),
            Resolver::Missing(residual) => std::ops::ControlFlow::Break(residual),
            Resolver::Account() => std::ops::ControlFlow::Break(MissingAccounts {
                accounts: vec![],
                address_lookup_tables: vec![],
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use solana_sha256_hasher::hashv;
    //
    #[test]
    fn test_resolver_discriminators_match() {
        // https://github.com/solana-program/libraries/blob/fcd6052feccb74b5ae4f7a8a7858e85d7f4adc93/discriminator/src/discriminator.rs#L40-L42
        let hash_bytes = hashv(&[RESOLVER_EXECUTE_VAA_V1_SEED]).to_bytes();
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
        assert_eq!(RESOLVER_EXECUTE_VAA_V1, discriminator_bytes);
    }
}
