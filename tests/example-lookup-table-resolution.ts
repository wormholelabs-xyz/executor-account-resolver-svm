import { splDiscriminate } from "@solana/spl-type-length-value";
import {
  AccountMeta,
  AddressLookupTableAccount,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ExampleLookupTableResolution } from "../target/types/example_lookup_table_resolution";
import { expect } from "chai";
import { decode } from "@coral-xyz/anchor/dist/cjs/utils/bytes/base64";
import { IdlCoder } from "@coral-xyz/anchor/dist/cjs/coder/borsh/idl";
import { resolveInstructions } from "./utils";

describe("example-lookup-table-resolution", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .ExampleLookupTableResolution as Program<ExampleLookupTableResolution>;

  const payer = anchor.web3.Keypair.generate();

  it("Is initialized!", async () => {
    // airdrop to payer
    await anchor
      .getProvider()
      .connection.confirmTransaction(
        await anchor
          .getProvider()
          .connection.requestAirdrop(payer.publicKey, 1000000000),
        "confirmed"
      );

    const recentSlot = (await program.provider.connection.getSlot()) - 1;
    const tx = await program.methods
      .initialize(new anchor.BN(recentSlot))
      .accounts({
        payer: program.provider.publicKey,
      })
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({
          units: 1_000_000,
        }),
      ])
      .rpc();
    console.log("LUT initialization signature", tx);
    // wait for lut to warm up
    await new Promise((resolve) => setTimeout(resolve, 2000));
    const result = await resolveInstructions(
      program.provider,
      program.programId,
      payer
    );
    for (const group of result) {
      for (const instruction of group.instructions) {
        console.log(instruction);

        let payerConst = new anchor.web3.PublicKey(
          Buffer.from("payer_00000000000000000000000000")
        );

        let accountsWithPayerOverride = instruction.accounts.map((account) => {
          if (account.pubkey.equals(payerConst)) {
            return { ...account, pubkey: payer.publicKey };
          }
          return account;
        });
        const ix = new anchor.web3.TransactionInstruction({
          keys: accountsWithPayerOverride,
          programId: instruction.programId,
          data: instruction.data,
        });
        const luts = (
          await Promise.all(
            group.addressLookupTables.map((lut) =>
              program.provider.connection.getAddressLookupTable(lut)
            )
          )
        ).map((r) => r.value);
        let { blockhash, lastValidBlockHeight } =
          await program.provider.connection.getLatestBlockhash();
        const messageV0 = new anchor.web3.TransactionMessage({
          payerKey: payer.publicKey,
          instructions: [
            ix,
            ComputeBudgetProgram.setComputeUnitLimit({
              units: 1_000_000,
            }),
          ],
          recentBlockhash: blockhash,
        }).compileToV0Message(luts); // if you remove luts here, you can see the transaction fail with `encoding overruns Uint8Array`
        const tx = new anchor.web3.VersionedTransaction(messageV0);
        tx.sign([payer]);
        let signature = await program.provider.connection.sendTransaction(tx);
        await program.provider.connection.confirmTransaction(
          { signature, blockhash, lastValidBlockHeight },
          "confirmed"
        );
      }
    }
  });

  it("derives the right discriminators", async () => {
    const expectedBytes = Buffer.from([148, 184, 169, 222, 207, 8, 154, 127]);
    const discriminator = await splDiscriminate(
      "executor-account-resolver:execute-vaa-v1"
    );
    expect(expectedBytes).to.deep.equal(discriminator);
    expect(
      program.idl.instructions.find((x) => x.name === "resolveExecuteVaaV1")
        .discriminator
    ).to.deep.equal([...expectedBytes]);
  });
});
