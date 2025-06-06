#![feature(try_trait_v2)]
use std::ops::{FromResidual, Try};

use anchor_lang::{prelude::*, solana_program::instruction::Instruction, Bumps};

pub const RESOLVER_EXECUTE_VAA_V1: [u8; 8] = [148, 184, 169, 222, 207, 8, 154, 127];

#[derive(AnchorSerialize)]
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

#[derive(AnchorSerialize)]
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

#[derive(AnchorSerialize)]
pub struct GroupsOf<T: AnchorSerialize>(pub Vec<Vec<T>>);

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
        Resolver::Missing(vec![key])
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

#[derive(AnchorSerialize)]
pub enum Resolver<T> {
    Resolved(T),
    Missing(Vec<Pubkey>),
}

impl<T> Resolver<T> {
    pub fn pair<U>(self, other: Resolver<U>) -> Resolver<(T, U)> {
        pair(self, other)
    }
}

pub fn pair<T, U>(a: Resolver<T>, b: Resolver<U>) -> Resolver<(T, U)> {
    match (a, b) {
        (Resolver::Resolved(a), Resolver::Resolved(b)) => Resolver::Resolved((a, b)),
        (Resolver::Resolved(_), Resolver::Missing(missing)) => Resolver::Missing(missing),
        (Resolver::Missing(missing), Resolver::Resolved(_)) => Resolver::Missing(missing),
        (Resolver::Missing(mut missing_a), Resolver::Missing(missing_b)) => {
            missing_a.extend(missing_b);
            Resolver::Missing(missing_a)
        }
    }
}

impl<T> FromResidual for Resolver<T> {
    fn from_residual(residual: Vec<Pubkey>) -> Self {
        Resolver::Missing(residual)
    }
}

impl<T> Try for Resolver<T> {
    type Output = T;

    type Residual = Vec<Pubkey>;

    fn from_output(output: Self::Output) -> Self {
        Resolver::Resolved(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Resolver::Resolved(output) => std::ops::ControlFlow::Continue(output),
            Resolver::Missing(residual) => std::ops::ControlFlow::Break(residual),
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
        let hash_input = "executor-account-resolver:execute-vaa-v1";
        let hash_bytes = hashv(&[hash_input.as_bytes()]).to_bytes();
        let mut discriminator_bytes = [0u8; 8];
        discriminator_bytes.copy_from_slice(&hash_bytes[..8]);
        assert_eq!(RESOLVER_EXECUTE_VAA_V1, discriminator_bytes);
    }
}
