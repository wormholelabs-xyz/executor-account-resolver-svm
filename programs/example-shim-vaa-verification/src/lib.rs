use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use anchor_lang::solana_program::{instruction::AccountMeta, keccak, pubkey};
use executor_account_resolver_svm::{
    missing_account, InstructionGroup, InstructionGroups, Resolver,
    RESOLVER_EXECUTE_VAA_V1, RESOLVER_PUBKEY_GUARDIAN_SET, RESOLVER_PUBKEY_SHIM_VAA_SIGS,
};
use wormhole_svm_definitions::{GUARDIAN_SET_SEED, zero_copy::GuardianSet};

declare_id!("EJqbvws7tkP7twXZQZvENi1qRiJEqKeP5NTmzvihoPsZ");

/// Verify-VAA shim program ID
const VERIFY_VAA_SHIM_PROGRAM_ID: Pubkey =
    pubkey!("EFaNWErqAtVWufdNb7yofSHHfWFos843DFpu4JBw24at");

/// Wormhole Core Bridge program ID (mainnet)
const WORMHOLE_CORE_PROGRAM_ID: Pubkey =
    pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");


#[program]
pub mod example_shim_vaa_verification {
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
        // Parse VAA body to get message hash and payload
        let (message_hash, payload_data) = parse_vaa_body(&vaa_body)?;

        // First run - request the guardian set account
        if ctx.remaining_accounts.is_empty() {
            return Ok(missing_account(RESOLVER_PUBKEY_GUARDIAN_SET))
        }

        // Second run - iterate through remaining accounts to find a valid guardian set
        let remaining_accounts_iter = &mut ctx.remaining_accounts.iter();
        let mut guardian_set_bump: Option<u8> = None;
        let mut guardian_set_pubkey : Option<Pubkey> = None;

        // Try to deserialize each remaining account as a GuardianSet
        while let Some(account_info) = remaining_accounts_iter.next() {
            // Try to deserialize as GuardianSet
            if let Ok(data) = account_info.try_borrow_data() {
                if let Some(guardian_set_account) = GuardianSet::new(&data) {
                    let guardian_set_index: u32 = guardian_set_account.guardian_set_index();
                    let (derived_guardian_set_pubkey, guardian_bump) = Pubkey::find_program_address(
                        &[GUARDIAN_SET_SEED, &guardian_set_index.to_be_bytes()],
                        &WORMHOLE_CORE_PROGRAM_ID
                    );

                    // Verify the account matches the expected PDA
                    if &derived_guardian_set_pubkey == account_info.key {
                        guardian_set_bump = Some(guardian_bump);
                        guardian_set_pubkey = Some(derived_guardian_set_pubkey);
                        break;
                    }
                }
            }
        }

        // If no valid guardian set was found, request it again
        let (guardian_set_bump, guardian_set_pubkey) = match (guardian_set_bump, guardian_set_pubkey) {
            (Some(bump), Some(pubkey)) => (bump, pubkey),
            _ => return Ok(missing_account(RESOLVER_PUBKEY_GUARDIAN_SET)),
        };

        // Parse payload (example only - not building real instructions from it)
        // In production, you would parse the payload to determine what instructions to build
        let _parsed_payload = ExampleParsedPayload::try_from(&payload_data[..])?;

        // Build the verify_hash instruction to call the verify-vaa shim
        // We use the guardian_set_bump that we derived above
        let verify_hash_ix = build_verify_hash_instruction(guardian_set_pubkey, guardian_set_bump, message_hash);

        // Return resolved instruction group with the verify_hash instruction
        // The guardian_set_pubkey is the actual resolved account we found in remaining_accounts
        // The executor will replace the RESOLVER_PUBKEY_SHIM_VAA_SIGS placeholder with the
        // guardian_signatures account (pre-posted by executor via PostSignatures)
        Ok(Resolver::Resolved(InstructionGroups(vec![
            InstructionGroup {
                instructions: vec![verify_hash_ix.into()],
                address_lookup_tables: vec![],
            },
        ])))
    }
}

/// Parse VAA body to extract necessary information.
///
/// The VAA body does not include the VAA header (which contains guardian set index
/// and signatures). The executor handles the header off-chain.
///
/// # Arguments
/// * `vaa_body` - The VAA body (without header)
///
/// # Returns
/// * `message_hash` - keccak256 hash of the VAA body (used for verification)
/// * `payload` - The payload data (for example parsing)
fn parse_vaa_body(vaa_body: &[u8]) -> Result<([u8; 32], Vec<u8>)> {
    require!(!vaa_body.is_empty(), ErrorCode::InvalidVaaBody);

    // Compute message hash (keccak256 of the body)
    // For V1 VAAs, the digest is: keccak256(keccak256(vaa_body))
    // But for simplicity in this example, we'll just use keccak256(vaa_body)
    let message_hash = keccak::hashv(&[vaa_body]).to_bytes();

    // In production, you would parse the VAA body structure to extract the actual payload
    // VAA body structure: timestamp (4) + nonce (4) + emitter_chain (2) + emitter_address (32)
    //                     + sequence (8) + consistency_level (1) + payload (variable)
    let min_body_size = 4 + 4 + 2 + 32 + 8 + 1;
    require!(
        vaa_body.len() >= min_body_size,
        ErrorCode::InvalidVaaBody
    );

    // Extract payload (everything after the 51 byte header)
    let payload = vaa_body[min_body_size..].to_vec();

    Ok((message_hash, payload))
}

/// Build the verify_hash instruction that calls the verify-vaa shim.
///
/// This instruction will be executed by the executor after the RESOLVER_PUBKEY_SHIM_VAA_SIGS
/// placeholder is replaced. The verify_hash instruction validates VAA signatures against a guardian set.
///
/// # Arguments
/// * `guardian_set_pubkey` - The resolved guardian set account pubkey
/// * `guardian_set_bump` - PDA bump seed for the guardian set account
/// * `message_hash` - keccak256 hash of the VAA body to verify
///
/// # Returns
/// Instruction that calls the verify-vaa shim's verify_hash endpoint
fn build_verify_hash_instruction(guardian_set_pubkey: Pubkey, guardian_set_bump: u8, message_hash: [u8; 32]) -> Instruction {
    // Build instruction data: discriminator + guardian_set_bump + message_hash
    let mut data = Vec::new();
    // verify_hash discriminator (from shim IDL)
    data.extend_from_slice(&[22, 152, 160, 69, 241, 148, 14, 124]);
    // guardian_set_bump: u8
    data.push(guardian_set_bump);
    // message_hash: [u8; 32]
    data.extend_from_slice(&message_hash);

    Instruction {
        program_id: VERIFY_VAA_SHIM_PROGRAM_ID,
        accounts: vec![
            // Guardian set account - resolved via Resolver::Missing
            AccountMeta {
                pubkey: guardian_set_pubkey,
                is_writable: false,
                is_signer: false,
            },
            // Guardian signatures account - populated by executor (NOT resolved)
            // The executor pre-posts signatures via the shim's PostSignatures instruction
            AccountMeta {
                pubkey: RESOLVER_PUBKEY_SHIM_VAA_SIGS,
                is_writable: true,
                is_signer: false,
            },
        ],
        data,
    }
}

/// Example payload struct (simplified for demonstration).
///
/// In production, this would parse according to your specific payload format.
/// This example just demonstrates the pattern without building real instructions.
#[allow(dead_code)]
struct ExampleParsedPayload {
    recipient: Pubkey,
    amount: u64,
}

impl TryFrom<&[u8]> for ExampleParsedPayload {
    type Error = anchor_lang::error::Error;

    fn try_from(payload_data: &[u8]) -> Result<Self> {
        // This is placeholder parsing logic
        // In production, you'd parse according to your payload format
        require!(payload_data.len() >= 40, ErrorCode::InvalidPayload);

        let recipient = Pubkey::try_from(&payload_data[0..32])
            .map_err(|_| ErrorCode::InvalidPayload)?;
        let amount = u64::from_le_bytes(payload_data[32..40].try_into().unwrap());

        Ok(Self { recipient, amount })
    }
}

/// Empty account structure for the resolver.
///
/// All necessary accounts are passed via `ctx.remaining_accounts`.
/// The guardian set account is resolved iteratively via `Resolver::Missing`.
#[derive(Accounts)]
pub struct Resolve {}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid VAA body")]
    InvalidVaaBody,
    #[msg("Invalid payload")]
    InvalidPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vaa_body_parsing() {
        // Create a minimal VAA body (51 bytes header + payload)
        let mut vaa_body = vec![0u8; 51];
        // Add some payload data
        vaa_body.extend_from_slice(b"test payload");

        let result = parse_vaa_body(&vaa_body);
        assert!(result.is_ok());

        let (message_hash, payload) = result.unwrap();
        assert_eq!(message_hash.len(), 32);
        assert_eq!(payload, b"test payload");
    }

    #[test]
    fn test_vaa_body_parsing_too_short() {
        // VAA body that's too short (< 51 bytes)
        let vaa_body = vec![0u8; 50];

        let result = parse_vaa_body(&vaa_body);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_verify_hash_instruction() {
        // Create a test guardian set pubkey (using a PDA derived from index 3)
        let guardian_set_index = 3u32;
        let (test_guardian_set_pubkey, guardian_set_bump) = Pubkey::find_program_address(
            &[GUARDIAN_SET_SEED, &guardian_set_index.to_be_bytes()],
            &WORMHOLE_CORE_PROGRAM_ID
        );
        let message_hash = [42u8; 32];

        let ix = build_verify_hash_instruction(test_guardian_set_pubkey, guardian_set_bump, message_hash);

        // Verify program ID
        assert_eq!(ix.program_id, VERIFY_VAA_SHIM_PROGRAM_ID);

        // Verify accounts
        assert_eq!(ix.accounts.len(), 2);
        assert_eq!(ix.accounts[0].pubkey, test_guardian_set_pubkey);
        assert_eq!(ix.accounts[1].pubkey, RESOLVER_PUBKEY_SHIM_VAA_SIGS);

        // Verify instruction data structure
        // discriminator (8) + bump (1) + hash (32) = 41 bytes
        assert_eq!(ix.data.len(), 41);

        // Verify discriminator
        assert_eq!(&ix.data[0..8], &[22, 152, 160, 69, 241, 148, 14, 124]);

        // Verify bump
        assert_eq!(ix.data[8], guardian_set_bump);

        // Verify hash
        assert_eq!(&ix.data[9..41], &message_hash);
    }

    #[test]
    fn test_example_payload_parsing() {
        // Create a valid payload (32 bytes pubkey + 8 bytes amount)
        let mut payload = vec![0u8; 32];
        payload.extend_from_slice(&1000u64.to_le_bytes());

        let result = ExampleParsedPayload::try_from(&payload[..]);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.amount, 1000);
    }

    #[test]
    fn test_example_payload_parsing_too_short() {
        // Payload that's too short
        let payload = vec![0u8; 30];

        let result = ExampleParsedPayload::try_from(&payload[..]);
        assert!(result.is_err());
    }
}
