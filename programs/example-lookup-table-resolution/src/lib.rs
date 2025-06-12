use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    address_lookup_table,
    entrypoint::MAX_PERMITTED_DATA_INCREASE,
    instruction::Instruction,
    program::{invoke, invoke_signed},
    system_instruction::MAX_PERMITTED_DATA_LENGTH,
};
use anchor_lang::{system_program, InstructionData};
use executor_account_resolver_svm::{
    InstructionGroup, InstructionGroups, MissingAccounts, Resolver, RESOLVER_EXECUTE_VAA_V1,
    RESOLVER_PUBKEY_PAYER, RESOLVER_RESULT_ACCOUNT, RESOLVER_RESULT_ACCOUNT_INIT_SIZE,
    RESOLVER_RESULT_ACCOUNT_SEED,
};

declare_id!("v3pcEfuzsPBGQ8Zy1jvtWq4iwugEWC2f3xgPd32eZgQ");

#[program]
pub mod example_lookup_table_resolution {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, recent_slot: u64) -> Result<()> {
        let (ix, lut_address) = address_lookup_table::instruction::create_lookup_table(
            ctx.accounts.authority.key(),
            ctx.accounts.payer.key(),
            recent_slot,
        );

        // just a sanity check, should never be hit, so we don't provide a custom
        // error message
        assert_eq!(lut_address, ctx.accounts.lut_address.key());

        // store the LUT
        ctx.accounts.lut.set_inner(LUT {
            bump: ctx.bumps.lut,
            address: lut_address,
        });

        // NOTE: LUTs can be permissionlessly created (i.e. the authority does
        // not need to sign the transaction). This means that the LUT might
        // already exist (if someone frontran us). However, it's not a problem:
        // AddressLookupTable::create_lookup_table checks if the LUT already
        // exists and does nothing if it does.
        //
        // LUTs can only be created permissionlessly, but only the authority is
        // authorised to actually populate the fields, so we don't have to worry
        // about the frontrunner populating it with junk. The only risk of that would
        // be the LUT being filled to capacity (256 addresses), with no
        // possibility for us to add our own accounts -- no other security impact.
        invoke(
            &ix,
            &[
                ctx.accounts.lut_address.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // write 128 accounts to the lookup table
        for i in 0..8 {
            let mut entries = Vec::with_capacity(16);
            for n in 0..16 {
                entries.push(Pubkey::find_program_address(&[&[(i * 16) + n]], &ID).0);
            }

            let ix = address_lookup_table::instruction::extend_lookup_table(
                ctx.accounts.lut_address.key(),
                ctx.accounts.authority.key(),
                Some(ctx.accounts.payer.key()),
                entries,
            );

            invoke_signed(
                &ix,
                &[
                    ctx.accounts.lut_address.to_account_info(),
                    ctx.accounts.authority.to_account_info(),
                    ctx.accounts.payer.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
                &[&[b"lut_authority", &[ctx.bumps.authority]]],
            )?;
        }

        Ok(())
    }

    pub fn execute_vaa_v1(_ctx: Context<ExecuteVAAv1>) -> Result<()> {
        Ok(())
    }

    #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
    pub fn resolve_execute_vaa_v1<'c: 'info, 'info>(
        ctx: Context<'_, '_, 'c, 'info, Resolve>,
        _vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        let result_pubkey = Pubkey::find_program_address(&[RESOLVER_RESULT_ACCOUNT_SEED], &ID).0;
        if ctx.remaining_accounts.is_empty() {
            // first run, we need our lut pointer, result account, payer for increasing the size of the result, and system program for transferring lamports
            return Ok(Resolver::Missing(MissingAccounts {
                accounts: vec![
                    Pubkey::find_program_address(&[b"lut"], &ID).0,
                    result_pubkey,
                    RESOLVER_PUBKEY_PAYER,
                    system_program::ID,
                ],
                address_lookup_tables: vec![],
            }));
        }
        if ctx.remaining_accounts.len() == 4 || ctx.remaining_accounts.len() == 68 {
            // second run and third runs, we need to read from the remaining accounts and write the result to the return account
            // this allows us to have an up to 10MiB response (if the account was pre-allocated) or up to 10KiB from an initialized account (due to the MAX_PERMITTED_DATA_INCREASE limit in a single transaction)

            // remaining accounts example
            // https://github.com/solana-foundation/anchor/blob/0bdfa3f760635cc83bbda13f9a9d22d1558d1776/tests/misc/programs/remaining-accounts/src/lib.rs#L33
            let remaining_accounts_iter = &mut ctx.remaining_accounts.iter();
            let lut_account =
                Account::<LUT>::try_from(next_account_info(remaining_accounts_iter)?)?;
            let ret_account_info = next_account_info(remaining_accounts_iter)?;
            let payer_info = next_account_info(remaining_accounts_iter)?;
            let system_program_info = next_account_info(remaining_accounts_iter)?;
            require_eq!(ret_account_info.is_writable, true);
            let mut ret = Account::<ExecutorAccountResolverResult>::try_from(ret_account_info)?;

            // increase the size of the return account
            let new_size = usize::min(
                ret_account_info.data_len() + MAX_PERMITTED_DATA_INCREASE,
                MAX_PERMITTED_DATA_LENGTH.try_into()?,
            );
            let lamports_diff = Rent::get().map(|rent| {
                rent.minimum_balance(new_size)
                    .saturating_sub(ret_account_info.lamports())
            })?;
            system_program::transfer(
                CpiContext::new(
                    system_program_info.to_account_info(),
                    system_program::Transfer {
                        from: payer_info.to_account_info(),
                        to: ret_account_info.to_account_info(),
                    },
                ),
                lamports_diff,
            )?;
            ret_account_info.realloc(new_size, false)?;

            if ctx.remaining_accounts.len() == 4 {
                // second run

                // calculate the accounts we need for the next iteration
                let mut entries = Vec::with_capacity(64);

                for n in 0..64 {
                    entries.push(Pubkey::find_program_address(&[&[n]], &ID).0);
                }

                // set the return value
                ret.set_inner(ExecutorAccountResolverResult(Resolver::Missing(
                    MissingAccounts {
                        accounts: entries,
                        address_lookup_tables: vec![lut_account.address],
                    },
                )));
                ret.exit(ctx.program_id)?;

                return Ok(Resolver::Account());
            } else if ctx.remaining_accounts.len() == 68 {
                // last run

                let instruction = Instruction {
                    program_id: crate::ID,
                    accounts: accounts::ExecuteVAAv1 {
                        payer: RESOLVER_PUBKEY_PAYER,
                        a1: Pubkey::find_program_address(&[&[64]], &ID).0,
                        a2: Pubkey::find_program_address(&[&[65]], &ID).0,
                        a3: Pubkey::find_program_address(&[&[66]], &ID).0,
                        a4: Pubkey::find_program_address(&[&[67]], &ID).0,
                        a5: Pubkey::find_program_address(&[&[68]], &ID).0,
                        a6: Pubkey::find_program_address(&[&[69]], &ID).0,
                        a7: Pubkey::find_program_address(&[&[70]], &ID).0,
                        a8: Pubkey::find_program_address(&[&[71]], &ID).0,
                        a9: Pubkey::find_program_address(&[&[72]], &ID).0,
                        a10: Pubkey::find_program_address(&[&[73]], &ID).0,
                        a11: Pubkey::find_program_address(&[&[74]], &ID).0,
                        a12: Pubkey::find_program_address(&[&[75]], &ID).0,
                        a13: Pubkey::find_program_address(&[&[76]], &ID).0,
                        a14: Pubkey::find_program_address(&[&[77]], &ID).0,
                        a15: Pubkey::find_program_address(&[&[78]], &ID).0,
                        a16: Pubkey::find_program_address(&[&[79]], &ID).0,
                        a17: Pubkey::find_program_address(&[&[80]], &ID).0,
                        a18: Pubkey::find_program_address(&[&[81]], &ID).0,
                        a19: Pubkey::find_program_address(&[&[82]], &ID).0,
                        a20: Pubkey::find_program_address(&[&[83]], &ID).0,
                        a21: Pubkey::find_program_address(&[&[84]], &ID).0,
                        a22: Pubkey::find_program_address(&[&[85]], &ID).0,
                        a23: Pubkey::find_program_address(&[&[86]], &ID).0,
                        a24: Pubkey::find_program_address(&[&[87]], &ID).0,
                        a25: Pubkey::find_program_address(&[&[88]], &ID).0,
                        a26: Pubkey::find_program_address(&[&[89]], &ID).0,
                        a27: Pubkey::find_program_address(&[&[90]], &ID).0,
                        a28: Pubkey::find_program_address(&[&[91]], &ID).0,
                        a29: Pubkey::find_program_address(&[&[92]], &ID).0,
                        a30: Pubkey::find_program_address(&[&[93]], &ID).0,
                        a31: Pubkey::find_program_address(&[&[94]], &ID).0,
                        a32: Pubkey::find_program_address(&[&[95]], &ID).0,
                        system_program: system_program::ID,
                    }
                    .to_account_metas(None),
                    data: instruction::ExecuteVaaV1.data(),
                };

                // set the return value
                ret.set_inner(ExecutorAccountResolverResult(Resolver::Resolved(
                    InstructionGroups(vec![InstructionGroup {
                        instructions: vec![instruction.into()],
                        address_lookup_tables: vec![lut_account.address],
                    }]),
                )));
                ret.exit(ctx.program_id)?;

                return Ok(Resolver::Account());
            }
        }
        err!(MyError::InvalidAccounts)
    }
}

#[account]
#[derive(InitSpace)]
pub struct LUT {
    pub bump: u8,
    pub address: Pubkey,
}

#[account(discriminator = RESOLVER_RESULT_ACCOUNT)]
pub struct ExecutorAccountResolverResult(Resolver<InstructionGroups>);

#[account]
#[derive(InitSpace)]
pub struct DummyAccount {}

#[derive(Accounts)]
#[instruction(recent_slot: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + RESOLVER_RESULT_ACCOUNT_INIT_SIZE,
        seeds = [RESOLVER_RESULT_ACCOUNT_SEED],
        bump
    )]
    pub result: Account<'info, ExecutorAccountResolverResult>,

    #[account(
        seeds = [b"lut_authority"],
        bump
    )]
    /// CHECK: The seeds constraint enforces that this is the correct account.
    pub authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [authority.key().as_ref(), &recent_slot.to_le_bytes()],
        seeds::program = address_lookup_table::program::id(),
        bump
    )]
    /// CHECK: The seeds constraint enforces that this is the correct account.
    pub lut_address: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + LUT::INIT_SPACE,
        seeds = [b"lut"],
        bump
    )]
    pub lut: Account<'info, LUT>,

    #[account(
        address = address_lookup_table::program::id(),
        executable
    )]
    /// CHECK: address lookup table program (checked by instruction)
    pub lut_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteVAAv1<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[64]],
        bump
    )]
    pub a1: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[65]],
        bump
    )]
    pub a2: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[66]],
        bump
    )]
    pub a3: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[67]],
        bump
    )]
    pub a4: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[68]],
        bump
    )]
    pub a5: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[69]],
        bump
    )]
    pub a6: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[70]],
        bump
    )]
    pub a7: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[71]],
        bump
    )]
    pub a8: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[72]],
        bump
    )]
    pub a9: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[73]],
        bump
    )]
    pub a10: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[74]],
        bump
    )]
    pub a11: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[75]],
        bump
    )]
    pub a12: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[76]],
        bump
    )]
    pub a13: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[77]],
        bump
    )]
    pub a14: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[78]],
        bump
    )]
    pub a15: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[79]],
        bump
    )]
    pub a16: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[80]],
        bump
    )]
    pub a17: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[81]],
        bump
    )]
    pub a18: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[82]],
        bump
    )]
    pub a19: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[83]],
        bump
    )]
    pub a20: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[84]],
        bump
    )]
    pub a21: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[85]],
        bump
    )]
    pub a22: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[86]],
        bump
    )]
    pub a23: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[87]],
        bump
    )]
    pub a24: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[88]],
        bump
    )]
    pub a25: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[89]],
        bump
    )]
    pub a26: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[90]],
        bump
    )]
    pub a27: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[91]],
        bump
    )]
    pub a28: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[92]],
        bump
    )]
    pub a29: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[93]],
        bump
    )]
    pub a30: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[94]],
        bump
    )]
    pub a31: Box<Account<'info, DummyAccount>>,

    #[account(
        init,
        payer = payer,
        space = 8,
        seeds = [&[95]],
        bump
    )]
    pub a32: Box<Account<'info, DummyAccount>>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Resolve {}

#[error_code]
pub enum MyError {
    #[msg("Invalid accounts")]
    InvalidAccounts,
}
