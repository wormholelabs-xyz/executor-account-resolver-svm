import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { splDiscriminate } from "@solana/spl-type-length-value";
import { expect } from "chai";
import { ExampleIterativeResolution } from "../target/types/example_iterative_resolution";
import { resolveInstructions } from "./utils";

describe("example-iterative-executor-account-resolver", () => {
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

    const tx = await program.methods
      .initialize()
      .accounts({
        payer: program.provider.publicKey,
      })
      .rpc();
    console.log("Your transaction signature", tx);
    const result = await resolveInstructions(
      program.provider,
      program.programId,
      payer
    );
    for (const group of result) {
      // TODO: send whole group as tx etc.
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
