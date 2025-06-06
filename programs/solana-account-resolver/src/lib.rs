use anchor_lang::{prelude::*, solana_program::instruction::Instruction, InstructionData};
use executor_account_resolver_svm::{
    GroupsOf, RemainingAccounts, Resolver, SerializableInstruction, RESOLVER_EXECUTE_VAA_V1,
};

declare_id!("2JrXahZgppXqGUBETfTJign3TTVCFznDa5oxgthYuT69");

#[program]
pub mod solana_account_resolver {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.foo.set_inner(MyAccount { data: 1 });
        ctx.accounts.bar.set_inner(MyAccount { data: 2 });
        ctx.accounts.baz.set_inner(MyAccount { data: 3 });
        Ok(())
    }

    #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
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
