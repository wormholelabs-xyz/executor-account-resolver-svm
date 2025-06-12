use anchor_lang::{prelude::*, solana_program::instruction::Instruction, InstructionData};
use executor_account_resolver_svm::{
    find_account, missing_account, InstructionGroup, InstructionGroups, Resolver,
    RESOLVER_EXECUTE_VAA_V1, RESOLVER_PUBKEY_PAYER,
};

declare_id!("8mjNDtRMN7Sjq2ZVjCjKJUUaCfUdfZLoeYREmYs3yKSi");

#[program]
pub mod example_iterative_resolution {

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
        _vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        Ok(accounts_to_execute2(ctx))
    }

    pub fn example_instruction(ctx: Context<ExampleInstruction>) -> Result<()> {
        ctx.accounts.qux.set_inner(MyAccount { data: 4 });
        Ok(())
    }
}

pub fn accounts_to_execute2(ctx: Context<Resolve>) -> Resolver<InstructionGroups> {
    // This example iteratively loads the accounts, as it simulates a condition where looking up a subsequent account
    // relies on data within a previous account.
    //
    // See example-lookup-table-resolution for requesting and resolving multiple accounts at once
    // in addition to handling results larger than the instruction return size of 1024.
    let (foo_key, _) = Pubkey::find_program_address(&[b"foo"], &crate::ID);
    let foo = if let Some(acc_info) = find_account(ctx.remaining_accounts, foo_key) {
        MyAccount::try_deserialize(&mut &acc_info.data.borrow()[..]).unwrap()
    } else {
        return missing_account(foo_key);
    };
    let (bar_key, _) = Pubkey::find_program_address(&[b"bar", &[foo.data]], &crate::ID);
    let bar = if let Some(acc_info) = find_account(ctx.remaining_accounts, bar_key) {
        MyAccount::try_deserialize(&mut &acc_info.data.borrow()[..]).unwrap()
    } else {
        return missing_account(bar_key);
    };
    let (baz_key, _) = Pubkey::find_program_address(&[b"baz", &[bar.data]], &crate::ID);
    let baz = if let Some(acc_info) = find_account(ctx.remaining_accounts, baz_key) {
        MyAccount::try_deserialize(&mut &acc_info.data.borrow()[..]).unwrap()
    } else {
        return missing_account(baz_key);
    };
    let (qux_key, _) = Pubkey::find_program_address(&[b"qux", &[baz.data]], &crate::ID);
    let accs = accounts::ExampleInstruction {
        payer: RESOLVER_PUBKEY_PAYER,
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
    Resolver::Resolved(InstructionGroups(vec![InstructionGroup {
        instructions: vec![instruction.into()],
        address_lookup_tables: vec![],
    }]))
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
