declare_program!(wormhole_verify_vaa_shim);

use anchor_lang::{
    prelude::*,
    solana_program::{keccak, pubkey},
};
use executor_account_resolver_svm::{
    missing_account, InstructionGroup, InstructionGroups, Resolver, RESOLVER_EXECUTE_VAA_V1,
    RESOLVER_PUBKEY_GUARDIAN_SET, RESOLVER_PUBKEY_SHIM_VAA_SIGS,
};
use wormhole_svm_definitions::{zero_copy::GuardianSet, GUARDIAN_SET_SEED};
use wormhole_verify_vaa_shim::program::WormholeVerifyVaaShim;

declare_id!("4uYX1FAV3RUzJ88efJZof9FJn5QqKrRpxzHTm1qZchLR");

/// Wormhole Core Bridge program ID (mainnet)
const WORMHOLE_CORE_PROGRAM_ID: Pubkey = pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");

#[program]
pub mod example_shim_vaa_verification {
    use anchor_lang::{solana_program::instruction::Instruction, InstructionData};

    use crate::wormhole_verify_vaa_shim::cpi::accounts::VerifyHash;

    use super::*;

    /// Resolver instruction that demonstrates VAA verification pattern with iterative resolution.
    ///
    /// This example showcases:
    /// - Iterative account resolution for guardian set account
    /// - Using `RESOLVER_PUBKEY_GUARDIAN_SET` placeholder (resolved via Missing)
    /// - Using `RESOLVER_PUBKEY_SHIM_VAA_SIGS` placeholder (executor pre-posts signatures)
    /// - Building the real `verify_hash` instruction that calls the verify-vaa shim
    ///
    /// The resolver checks if the guardian set account is present in `ctx.remaining_accounts`.
    /// If not present, it returns `Resolver::Missing` to inform the executor to add it.
    /// If present, it reads the account data to get the bump and builds the instruction.
    ///
    /// # Arguments
    /// * `vaa_body` - The VAA body (without header). Used to compute message hash.
    /// * `guardian_set_index` - Guardian set index from VAA header (not in body)
    #[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
    pub fn resolve_execute_vaa_v1(
        ctx: Context<Resolve>,
        vaa_body: Vec<u8>,
    ) -> Result<Resolver<InstructionGroups>> {
        // Iterate through remaining accounts to find a valid guardian set
        let remaining_accounts_iter = &mut ctx.remaining_accounts.iter();
        let mut guardian_set_bump: Option<u8> = None;
        let mut guardian_set_pubkey: Option<Pubkey> = None;

        // Try to deserialize each remaining account as a GuardianSet
        while let Some(account_info) = remaining_accounts_iter.next() {
            // Try to deserialize as GuardianSet
            if let Ok(data) = account_info.try_borrow_data() {
                if let Some(guardian_set_account) = GuardianSet::new(&data) {
                    let guardian_set_index: u32 = guardian_set_account.guardian_set_index();
                    let (derived_guardian_set_pubkey, guardian_bump) = Pubkey::find_program_address(
                        &[GUARDIAN_SET_SEED, &guardian_set_index.to_be_bytes()],
                        &WORMHOLE_CORE_PROGRAM_ID,
                    );

                    // Verify the account matches the expected PDA
                    if *account_info.key == derived_guardian_set_pubkey {
                        guardian_set_bump = Some(guardian_bump);
                        guardian_set_pubkey = Some(derived_guardian_set_pubkey);
                        break;
                    }
                }
            }
        }

        // If no valid guardian set was found, request it
        let (guardian_set_bump, guardian_set_pubkey) =
            match (guardian_set_bump, guardian_set_pubkey) {
                (Some(bump), Some(pubkey)) => (bump, pubkey),
                _ => return Ok(missing_account(RESOLVER_PUBKEY_GUARDIAN_SET)),
            };

        // Build the execute_vaa_v1 instruction
        // We use the guardian_set_bump that we derived above
        let accs = accounts::ExecuteVAAv1 {
            guardian_set: guardian_set_pubkey,
            guardian_signatures: RESOLVER_PUBKEY_SHIM_VAA_SIGS,
            wormhole_verify_vaa_shim: WormholeVerifyVaaShim::id(),
        };
        let ix = Instruction {
            program_id: crate::ID,
            accounts: accs.to_account_metas(None),
            data: (instruction::ExecuteVaaV1 {
                guardian_set_bump: guardian_set_bump,
                vaa_body: vaa_body,
            })
            .data(),
        };

        // Return resolved instruction group with the execute_vaa_v1 instruction
        // The guardian_set_pubkey is the actual resolved account we found in remaining_accounts
        // The executor will replace the RESOLVER_PUBKEY_SHIM_VAA_SIGS placeholder with the
        // guardian_signatures account (pre-posted by executor via PostSignatures)
        Ok(Resolver::Resolved(InstructionGroups(vec![
            InstructionGroup {
                instructions: vec![ix.into()],
                address_lookup_tables: vec![],
            },
        ])))
    }

    pub fn execute_vaa_v1(
        ctx: Context<ExecuteVAAv1>,
        guardian_set_bump: u8,
        vaa_body: Vec<u8>,
    ) -> Result<()> {
        // Compute the message hash.
        let message_hash = keccak::hashv(&[&vaa_body]).to_bytes();
        let digest = keccak::hash(message_hash.as_slice()).to_bytes();
        wormhole_verify_vaa_shim::cpi::verify_hash(
            CpiContext::new(
                ctx.accounts.wormhole_verify_vaa_shim.to_account_info(),
                VerifyHash {
                    guardian_set: ctx.accounts.guardian_set.to_account_info(),
                    guardian_signatures: ctx.accounts.guardian_signatures.to_account_info(),
                },
            ),
            guardian_set_bump,
            digest,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Resolve {}

#[derive(Accounts)]
pub struct ExecuteVAAv1<'info> {
    /// CHECK: Guardian set used for signature verification by shim.
    /// Derivation is checked by the shim.
    guardian_set: UncheckedAccount<'info>,

    /// CHECK: Stores guardian signatures to be verified by shim.
    guardian_signatures: UncheckedAccount<'info>,

    wormhole_verify_vaa_shim: Program<'info, WormholeVerifyVaaShim>,
}
