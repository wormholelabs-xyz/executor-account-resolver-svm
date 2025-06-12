use anchor_lang::{prelude::*, solana_program::instruction::Instruction};

// NOTE: The `AnchorSerialize`d structs in this file MUST NOT break existing serialization/deserialization
// compatibility as used by an instruction described in the spec and used in production.
// This means that if any type needs to change, it instead must be duplicated, modified,
// and added as a new enum variant in `Resolver`.

// hash inputs
/// The hash input for `RESOLVER_EXECUTE_VAA_V1`.
pub const RESOLVER_EXECUTE_VAA_V1_SEED: &[u8] = b"executor-account-resolver:execute-vaa-v1";
/// The PDA seed for calculating the return account.
pub const RESOLVER_RESULT_ACCOUNT_SEED: &[u8] = b"executor-account-resolver:result";
/// The initial size for an empty resolver result `Resolver::Resolved(InstructionGroups(vec![]))`
///
/// Usage: `space = 8 + RESOLVER_RESULT_ACCOUNT_INIT_SIZE`
pub const RESOLVER_RESULT_ACCOUNT_INIT_SIZE: usize = 5;

// discriminators
/// Discriminator for resolving the instructions for executing a v1 VAA.
///
/// Usage:
///
/// ```rust
/// use anchor_lang::prelude::*;
/// use executor_account_resolver_svm::{
///     InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1,
/// };
///
/// #[derive(Accounts)]
/// pub struct Resolve {}
///
/// #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
/// pub fn resolve_execute_vaa_v1(ctx: Context<Resolve>, vaa_body: Vec<u8>) -> Result<Resolver<InstructionGroups>> {
///     Ok(Resolver::Resolved(InstructionGroups(vec![
///         // build your `InstructionGroup`s here
///     ])))
/// }
/// ```
///
/// Ensure that you have the `interface-instructions` feature enabled.
pub const RESOLVER_EXECUTE_VAA_V1: [u8; 8] = [148, 184, 169, 222, 207, 8, 154, 127];
/// Discriminator to be used for a resolver result account
///
/// Usage:
///
/// ```ignore
/// #[account(discriminator = RESOLVER_RESULT_ACCOUNT)]
/// pub struct ExecutorAccountResolverResult(Resolver<InstructionGroups>);
/// ```
pub const RESOLVER_RESULT_ACCOUNT: &[u8; 8] = &[34, 185, 243, 199, 181, 255, 28, 227];

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

// helpers
/// A helper function for finding a pubkey in `remaining_accounts`.
/// Depending on the use case, it may be more efficient to assume accounts at certain indexes.
///
/// Usage:
///
/// ```rust
/// use anchor_lang::prelude::*;
/// use executor_account_resolver_svm::{
///     find_account, missing_account, InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1,
/// };
///
/// #[derive(Accounts)]
/// pub struct Resolve {}
///
/// #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
/// pub fn resolve_execute_vaa_v1(ctx: Context<Resolve>, vaa_body: Vec<u8>) -> Result<Resolver<InstructionGroups>> {
///     let mint = pubkey!("So11111111111111111111111111111111111111112");
///     let mint_info = if let Some(acc_info) = find_account(ctx.remaining_accounts, mint) {
///         acc_info
///     } else {
///         return Ok(missing_account(mint));
///     };
///     Ok(Resolver::Resolved(InstructionGroups(vec![
///         // build your `InstructionGroup`s here
///     ])))
/// }
/// ```
pub fn find_account<'c, 'info>(
    accs: &'c [AccountInfo<'info>],
    pubkey: Pubkey,
) -> Option<&'c AccountInfo<'info>> {
    accs.iter().find(|acc_info| *acc_info.key == pubkey)
}

/// A helper function for resolving to a single missing account.
///
/// Usage:
///
/// ```rust
/// use anchor_lang::prelude::*;
/// use executor_account_resolver_svm::{
///     find_account, missing_account, InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1,
/// };
///
/// #[derive(Accounts)]
/// pub struct Resolve {}
///
/// #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
/// pub fn resolve_execute_vaa_v1(ctx: Context<Resolve>, vaa_body: Vec<u8>) -> Result<Resolver<InstructionGroups>> {
///     let mint = pubkey!("So11111111111111111111111111111111111111112");
///     let mint_info = if let Some(acc_info) = find_account(ctx.remaining_accounts, mint) {
///         acc_info
///     } else {
///         return Ok(missing_account(mint));
///     };
///     Ok(Resolver::Resolved(InstructionGroups(vec![
///         // build your `InstructionGroup`s here
///     ])))
/// }
/// ```
///
/// Depending on the use case, it may be more efficient to resolve multiple missing accounts,
/// in which case `Resolver::Missing(MissingAccounts)` can be constructed manually.
///
pub fn missing_account(pubkey: Pubkey) -> Resolver<InstructionGroups> {
    Resolver::Missing(MissingAccounts {
        accounts: vec![pubkey],
        address_lookup_tables: vec![],
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use solana_sha256_hasher::hashv;

    #[test]
    fn test_resolver_discriminators_match() {
        {
            // https://github.com/solana-program/libraries/blob/fcd6052feccb74b5ae4f7a8a7858e85d7f4adc93/discriminator/src/discriminator.rs#L40-L42
            let hash_bytes = hashv(&[RESOLVER_EXECUTE_VAA_V1_SEED]).to_bytes();
            let mut discriminator_bytes = [0u8; 8];
            discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
            assert_eq!(discriminator_bytes, RESOLVER_EXECUTE_VAA_V1);
        }
        {
            let hash_bytes = hashv(&[RESOLVER_RESULT_ACCOUNT_SEED]).to_bytes();
            let mut discriminator_bytes = [0u8; 8];
            discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
            assert_eq!(discriminator_bytes, *RESOLVER_RESULT_ACCOUNT);
        }
    }

    #[test]
    fn test_resolver_resolved_empty_serialization() {
        let resolved = Resolver::Resolved(InstructionGroups(vec![]));
        let mut buffer: Vec<u8> = Vec::new();
        resolved.serialize(&mut buffer).unwrap();
        assert_eq!(buffer.len(), RESOLVER_RESULT_ACCOUNT_INIT_SIZE);
        assert_eq!(buffer, [0, 0, 0, 0, 0])
    }

    #[test]
    fn test_resolver_missing_empty_serialization() {
        let missing: Resolver<InstructionGroups> = Resolver::Missing(MissingAccounts {
            accounts: vec![],
            address_lookup_tables: vec![],
        });
        let mut buffer: Vec<u8> = Vec::new();
        missing.serialize(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 9);
        assert_eq!(buffer, [1, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    #[test]
    fn test_resolver_account_serialization() {
        let account: Resolver<InstructionGroups> = Resolver::Account();
        let mut buffer: Vec<u8> = Vec::new();
        account.serialize(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer, [2])
    }
}
