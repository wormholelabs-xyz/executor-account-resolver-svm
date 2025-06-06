import { splDiscriminate } from "@solana/spl-type-length-value";
import {
  AccountMeta,
  AddressLookupTableAccount,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ExampleIterativeResolution } from "../target/types/example_iterative_resolution";
import { expect } from "chai";
import { decode } from "@coral-xyz/anchor/dist/cjs/utils/bytes/base64";
import { IdlCoder } from "@coral-xyz/anchor/dist/cjs/coder/borsh/idl";
import { resolveInstructions } from "./utils";

describe("solana-account-resolver", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .ExampleIterativeResolution as Program<ExampleIterativeResolution>;

  const payer = anchor.web3.Keypair.generate();
  // airdrop to payer
  it("Is initialized!", async () => {
    await anchor
      .getProvider()
      .connection.confirmTransaction(
        await anchor
          .getProvider()
          .connection.requestAirdrop(payer.publicKey, 1000000000),
        "confirmed"
      );

    // const [bar, _] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("bar"), Buffer.from([10])], program.programId);
    // const [bar, _] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("bar"), Buffer.from([10])], program.programId);
    const tx = await program.methods
      .initialize()
      .accounts({
        payer: program.provider.publicKey,
        // foo: fooKeypair.publicKey,
        // bar: bar
      })
      .rpc();
    console.log("Your transaction signature", tx);
    try {
      const result = await resolveInstructions(
        program.provider,
        program.programId,
        payer
      );
      for (const group of result) {
        // TODO: send whole group as tx etc.
        for (const instruction of group.instructions) {
          console.log(instruction);

          // 32 9s
          let payerConst = new anchor.web3.PublicKey(
            Buffer.from("payer_00000000000000000000000000")
          );

          let accountsWithPayerOverride = instruction.accounts.map(
            (account) => {
              if (account.pubkey.equals(payerConst)) {
                return { ...account, pubkey: payer.publicKey };
              }
              return account;
            }
          );
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
            instructions: [ix],
            recentBlockhash: blockhash,
          }).compileToV0Message(luts);
          const tx = new anchor.web3.VersionedTransaction(messageV0);
          tx.sign([payer]);
          let signature = await program.provider.connection.sendTransaction(tx);
          await program.provider.connection.confirmTransaction(
            { signature, blockhash, lastValidBlockHeight },
            "confirmed"
          );
        }
      }
    } catch (e) {
      console.log("Error", e);
    }
  });

  it("derives the right discriminators", async () => {
    const expectedBytes = Buffer.from([148, 184, 169, 222, 207, 8, 154, 127]);
    const discriminator = await splDiscriminate(
      "executor-account-resolver:execute-vaa-v1"
    );
    expect(expectedBytes).to.deep.equal(discriminator);
    expect(
      program.idl.instructions.find((x) => x.name === "accountsToExecute")
        .discriminator
    ).to.deep.equal([...expectedBytes]);
  });
});
