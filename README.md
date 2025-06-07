# Executor Account Resolver SVM

## Objective

Provide a mechanism for on-chain programs to describe the accounts and instructions necessary to perform an action based on a protocol message. The intention is to allow for nearly any form of generic message relaying, initially to support [Wormhole VAA v1](https://github.com/wormhole-foundation/wormhole/blob/main/whitepapers/0001_generic_message_passing.md) via [Executor](https://github.com/wormholelabs-xyz/example-messaging-executor).

## Background

Generic relaying on EVM can be accomplished by calling a pre-determined function with an established signature. However, the Solana Virtual Machine has several limitations that make this difficult.

- Transactions have a [size limit of 1232 bytes](https://solana.com/docs/core/transactions#key-points). This restricts the number of instruction accounts and data that can be used in a single transaction.
- Top-level instructions are restricted to a [CPI call depth of of 4](https://solana.com/docs/programs/limitations#cpi-call-depth---calldepth-error).
- Instructions require [explicitly listing the accounts](https://solana.com/docs/core/transactions#key-points) to be read from and written to.

For example, a Wormhole Token Bridge redeem takes several transactions.

1. Verify the signatures against the Wormhole Core Bridge - the 13 signatures on mainnet effectively require their own transaction or two.
2. Post the VAA to the Wormhole Core Bridge - this ensures the signatures are valid for the hash of VAA body and posts the body on-chain. This requires the signature account from the prior transaction.
3. Redeem the transfer on the Wormhole Token Bridge via one of two instructions: Native tokens are unlocked while foreign (wrapped) tokens are minted, requiring different sets of accounts. Today, this requires specialized off-chain SDK code to determine which instruction to call based on the contents of the VAA.

### Prior Art

[The SPL Transfer Hook Interface](https://www.solana-program.com/docs/transfer-hook-interface/specification) uses a specific discriminator to allow for limited custom transfer functionality. It also provides a mechanism to provide a number of additional accounts. Notably, it does not allow for any custom logic that would require different / arbitrary accounts based on the transfer being performed or logic that requires more than one instruction.

## Goals

- Create a mechanism that allows an off-chain service to discover how a program requires a protocol message to be relayed.
- Support arbitrary logic in resolving the necessary instructions.
- Support reading from arbitrary accounts during the resolution.
- Support resolving instructions that must be performed across multiple transactions.
- Support resolving instructions that require lookup tables in order to fit under the transaction limit.
- Support resolving a series of instructions and accounts that are cumulatively larger than the [return data limit of 1024 bytes](https://docs.anza.xyz/proposals/return-data).

## Non-Goals

- On-chain integration.

## Overview

Akin to the Transfer Hook Interface, define a set discriminator, instruction data, and accounts for each supported Executor request type starting with [VAA v1](https://github.com/wormholelabs-xyz/example-messaging-executor?tab=readme-ov-file#vaa-v1-request). Provide a Rust crate and example on-chain and off-chain code for integrators to use.

## Detailed Design

### Technical Details

#### On-Chain

In order to support results larger than the 1024 byte limit, a canonical result account PDA seed is defined as `executor-account-resolver:result`.

Since resolution may require reading from accounts in order to determine the instructions required (e.g. does the recipient's token account exist or what is the address of the canonical address lookup table), the resolution process itself must be iterative and require a mechanism to request further accounts. To support this, a `Resolver` enum is defined with the following fields:

- `Resolved`: The resolution is complete and the result is included.
- `Missing`: The resolution is incomplete and requires more accounts.
- `Account`: The Resolver result was written to the canonical result account.

Some accounts may not be deterministically known to the on-chain program and only able to be determined at execution time by the off-chain relayer. For these cases, the following account placeholders have been defined.

- `payer_00000000000000000000000000`: The public key of the relayer.
- `posted_vaa_000000000000000000000`: The Wormhole Core Bridge Posted VAA - this indicates to the off-chain relayer that the v1 VAA must first be posted to the Core Bridge.
- `shim_vaa_sigs_000000000000000000`: The [Wormhole Verify VAA Shim](https://github.com/wormhole-foundation/wormhole/blob/main/svm/wormhole-core-shims/programs/verify-vaa/README.md) Guardian Signatures account - this indicates to the off-chain relayer that the v1 VAA's signatures must first be posted to the Verify VAA Shim.
- `keypair_nn_000000000000000000000`: A new keypair generated by the relayer. `nn` is a placeholder used to uniquely identify the generated keypair's public key across multiple instructions. Constants for `00` through `09` are provided.

The result of a resolution has several nested structs.

- `InstructionGroups`: a vector of `InstructionGroup` - each group represents instructions that may need to be submitted as separate transaction due to transaction size or other limitations.
- `InstructionGroup`: contains a vector of `SerializableInstruction` and a vector of Address Lookup Table public keys.
- `SerializableInstruction`: a Solana `Instruction` that can be serialized by Anchor.
- `SerializableAccountMeta`: a Solana `AccountMeta` that can be serialized by Anchor.

The signature for VAA v1 resolution must look like the following:

```rust
#[derive(Accounts)]
pub struct Resolve {}

#[instruction(discriminator = &RESOLVER_EXECUTE_VAA_V1)]
pub fn resolve_execute_vaa_v1(
  _ctx: Context<Resolve>,
  _vaa_body: Vec<u8>,
) -> Result<Resolver<InstructionGroups>>
```

Accounts that are required for resolution can be requested like:

```rust
Ok(Resolver::Missing(MissingAccounts {
    accounts: vec![
        Pubkey::find_program_address(&[b"config"], &ID).0,
    ],
    address_lookup_tables: vec![],
}));
```

A completed resolution looks like:

```rust
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
```

See the examples for more use cases.

#### Off-Chain

Off-chain resolution must follow the above spec and generally follows this pattern:

1. Simulate a call to `resolveExecuteVaaV1` with the body of the VAA to be relayed and an empty array of `remainingAccounts` and compile a VersionedTransaction with an empty array of `lookupTables`. Include the canonical result account in the post simulation state to return.
2. Parse the return data. If the return is `Resolver::Account`, parse the account data included in the simulation result.
3. If the result was `Resolver::Missing`, append the specified accounts to `remainingAccounts`, resolve the lookup tables and append them to `lookupTables`, and repeat step 1.
4. Repeat until a set number of iterations have been exhausted or the result is `Resolver::Resolved`.

### Protocol Integration

This requires no Wormhole or Executor protocol changes.

### API / database schema

N/A

## Caveats

This design attempts to be fairly flexible in the supported complexity of use cases, but it is always possible that a Solana protocol update may change the expected limitations. It is recommended to use permissionless programs and design your integration to support changing the resolver or relay handler if needed. The above design allows for invoking instructions on any program, not just the same program that is performing the resolution.

Account-based return data quickly becomes complex for an integrator to manage. See Security Considerations.

Additional helpers can be added to `executor-account-resolver-svm` in the future and the `Resolver` enum can be extended but the existing serialization must not change.

## Alternatives Considered

Initially, only the return data was used, but real-world use cases quickly hit the limit.

## Security Considerations

Integrators who require account-based return data need to carefully manage the return data account.

- Accounts can only only be increased by `MAX_PERMITTED_DATA_INCREASE` within a transaction, which is `1_024 * 10` as of this writing. If a larger return is needed, the account will need to be permanently pre-allocated.
- Anyone can submit the resolver instruction on-chain, writing the result data to the result account. Therefore, the logic in the resolve function must appropriately handle this possibility. For example, do not always attempt to increase the account by `MAX_PERMITTED_DATA_INCREASE` as the `MAX_PERMITTED_DATA_LENGTH` (`10 * 1024 * 1024`) may not be exceeded.
- Stack limitations may affect the ability to construct large sets of instructions and accounts in memory. Writing the borsch serialization to the result account piecemeal may be required.

## Test Plan

Expected use cases should be covered by cargo and anchor tests.

## Performance Impact

This is the first approach, in the Wormhole ecosystem at least, to support generic relaying on SVM. The immediate performance issues to watch out for are the number of compute units required for resolution, the return data size required (including any lamports needed during simulation for its size changes), and defending against resolvers that infinite loop.

⚠ **This software is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
implied. See the License for the specific language governing permissions and limitations under the License.** Or plainly
spoken - this is a very complex piece of software which targets a bleeding-edge, experimental smart contract runtime.
Mistakes happen, and no matter how hard you try and whether you pay someone to audit it, it may eat your tokens, set
your printer on fire or startle your cat. Cryptocurrencies are a high-risk investment, no matter how fancy.
