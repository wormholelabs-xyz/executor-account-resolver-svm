import { splDiscriminate } from "@solana/spl-type-length-value";
import {
  AccountMeta,
  AddressLookupTableAccount,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaAccountResolver } from "../target/types/solana_account_resolver";
import { expect } from "chai";
import { decode } from "@coral-xyz/anchor/dist/cjs/utils/bytes/base64";
import { IdlCoder } from "@coral-xyz/anchor/dist/cjs/coder/borsh/idl";

describe("solana-account-resolver", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .SolanaAccountResolver as Program<SolanaAccountResolver>;

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
      const result = await resolveInstructions(program, payer);
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

type Instruction = {
  programId: anchor.web3.PublicKey;
  accounts: AccountMeta[];
  data: Buffer;
};

type InstructionGroup = {
  instructions: Instruction[];
  addressLookupTables: anchor.web3.PublicKey[];
};

// a function that calls accountsToExecute repeatedly until it returns ok. as
// long as it returns missing, we add the returned missing keys to
// remainingAccounts and call accountsToExecute again
async function resolveInstructions(
  program: Program<SolanaAccountResolver>,
  payerWallet: anchor.web3.Keypair
): Promise<InstructionGroup[]> {
  const remainingAccounts: AccountMeta[] = [];
  const luts: AddressLookupTableAccount[] = [];
  let runs = 0;
  while (true) {
    runs++;
    // support simulation with lookup table
    // adapted from https://github.com/solana-foundation/anchor/blob/0bdfa3f760635cc83bbda13f9a9d22d1558d1776/ts/packages/anchor/src/program/namespace/views.ts#L26C7-L42C39
    const ix = await program.methods
      .accountsToExecute()
      .remainingAccounts(remainingAccounts)
      .instruction();
    let { blockhash } = await program.provider.connection.getLatestBlockhash();
    const messageV0 = new anchor.web3.TransactionMessage({
      payerKey: payerWallet.publicKey,
      instructions: [ix],
      recentBlockhash: blockhash,
    }).compileToV0Message(luts);
    const tx = new anchor.web3.VersionedTransaction(messageV0);
    const simulationResult =
      await program.provider.connection.simulateTransaction(tx, {
        replaceRecentBlockhash: true,
      });
    const returnPrefix = `Program return: ${program.programId} `;
    let returnLog = simulationResult.value.logs.find((l) =>
      l.startsWith(returnPrefix)
    );
    if (!returnLog) {
      throw new Error("View expected return log");
    }

    let returnData = decode(returnLog.slice(returnPrefix.length));
    let returnType = program.idl.instructions.find(
      (i) => i.name === "accountsToExecute"
    ).returns;
    if (!returnType) {
      throw new Error("View expected return type");
    }

    const coder = IdlCoder.fieldLayout({ type: returnType }, program.idl.types);
    const result = coder.decode(returnData);
    console.log(JSON.stringify(result, undefined, 2));
    if (result.resolved) {
      console.log("Runs", runs);
      return result.resolved[0][0];
    } else {
      let newAccountMetas = result.missing[0].accounts.map((key) => {
        return { pubkey: key, isSigner: false, isWritable: false };
      });
      remainingAccounts.push(...newAccountMetas);
      let newLookupTables = (
        await Promise.all(
          result.missing[0].addressLookupTables.map((lut) =>
            program.provider.connection.getAddressLookupTable(lut)
          )
        )
      ).map((r) => r.value);
      luts.push(...newLookupTables);
    }
  }
}
