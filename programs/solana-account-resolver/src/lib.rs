#![feature(try_trait_v2)]
use std::ops::{FromResidual, Try};

use anchor_lang::{prelude::*, solana_program::instruction::Instruction, Bumps, InstructionData};

declare_id!("2JrXahZgppXqGUBETfTJign3TTVCFznDa5oxgthYuT69");

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
pub struct GroupsOf<T: AnchorSerialize>(Vec<Vec<T>>);

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

#[program]
pub mod solana_account_resolver {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.foo.set_inner(MyAccount { data: 1 });
        ctx.accounts.bar.set_inner(MyAccount { data: 2 });
        ctx.accounts.baz.set_inner(MyAccount { data: 3 });
        Ok(())
    }

    pub fn accounts_to_execute(
        ctx: Context<Resolve>,
    ) -> Result<Resolver<GroupsOf<SerializableInstruction>>> {
        Ok(accounts_to_execute2(ctx))
    }

    pub fn example_instruction(ctx: Context<ExampleInstruction>) -> Result<()> {
        ctx.accounts.qux.set_inner(MyAccount { data: 4 });
        Ok(())
    }
}

// TODO: better magic
const PAYER: Pubkey = Pubkey::new_from_array([9u8; 32]);

pub fn accounts_to_execute2(ctx: Context<Resolve>) -> Resolver<GroupsOf<SerializableInstruction>> {
    // TODO: use an example where we load an account but not necessarily use its
    // pubkey in the final thing (e.g. reading stuff off state)
    // TODO: example for parallel loading (to reduce number of roundtrips when we can)
    let ctx_accs = RemainingAccounts::from(ctx);
    let (foo_key, _) = Pubkey::find_program_address(&[b"foo"], &crate::ID);
    let foo = ctx_accs.load_deserialize::<MyAccount>(foo_key)?;
    let (bar_key, _) = Pubkey::find_program_address(&[b"bar", &[foo.data]], &crate::ID);
    let bar = ctx_accs.load_deserialize::<MyAccount>(bar_key)?;
    let (baz_key, _) = Pubkey::find_program_address(&[b"baz", &[bar.data]], &crate::ID);
    let baz = ctx_accs.load_deserialize::<MyAccount>(baz_key)?;
    let (qux_key, _) = Pubkey::find_program_address(&[b"qux", &[baz.data]], &crate::ID);
    let accs = accounts::ExampleInstruction {
        payer: PAYER,
        foo: foo_key,
        bar: bar_key,
        baz: baz_key,
        qux: qux_key,
        system_program: System::id(),
    };
    let instruction: Instruction = Instruction {
        program_id: crate::ID,
        accounts: accs.to_account_metas(None),
        data: (instruction::ExampleInstruction {}).data(),
    };
    Resolver::Resolved(GroupsOf(vec![vec![instruction.into()]]))
}

#[account]
#[derive(InitSpace)]
pub struct MyAccount {
    pub data: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + MyAccount::INIT_SPACE,
        seeds = [b"foo"],
        bump
    )]
    pub foo: Account<'info, MyAccount>,

    #[account(
        init,
        payer = payer,
        space = 8 + MyAccount::INIT_SPACE,
        seeds = [b"bar", 1u8.to_le_bytes().as_ref()],
        bump
    )]
    pub bar: Account<'info, MyAccount>,

    #[account(
        init,
        payer = payer,
        space = 8 + MyAccount::INIT_SPACE,
        seeds = [b"baz", 2u8.to_le_bytes().as_ref()],
        bump
    )]
    pub baz: Account<'info, MyAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExampleInstruction<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [b"foo"],
        bump
    )]
    pub foo: Account<'info, MyAccount>,

    #[account(
        seeds = [b"bar", &[foo.data]],
        bump
    )]
    pub bar: Account<'info, MyAccount>,

    #[account(
        seeds = [b"baz", &[bar.data]],
        bump
    )]
    pub baz: Account<'info, MyAccount>,

    #[account(
        init,
        space = 8 + MyAccount::INIT_SPACE,
        payer = payer,
        seeds = [b"qux".as_ref(), &[baz.data]],
        bump
    )]
    pub qux: Account<'info, MyAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Resolve {}
