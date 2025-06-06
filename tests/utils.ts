import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { IdlCoder } from "@coral-xyz/anchor/dist/cjs/coder/borsh/idl";
import { decode } from "@coral-xyz/anchor/dist/cjs/utils/bytes/base64";
import { AccountMeta, AddressLookupTableAccount } from "@solana/web3.js";
import { ExecutorAccountResolverSvmProgram } from "../target/types/executor_account_resolver_svm_program";
import ExecutorAccountResolverSvmProgramIdl from "../target/idl/executor_account_resolver_svm_program.json";

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
export async function resolveInstructions(
  provider: anchor.Provider,
  programId: anchor.web3.PublicKey,
  payerWallet: anchor.web3.Keypair
): Promise<InstructionGroup[]> {
  const overrideIdl = {
    ...ExecutorAccountResolverSvmProgramIdl,
    address: programId,
  };
  const program = new Program<ExecutorAccountResolverSvmProgram>(
    overrideIdl,
    provider
  );
  const remainingAccounts: AccountMeta[] = [];
  const luts: AddressLookupTableAccount[] = [];
  let runs = 0;
  while (true) {
    runs++;
    // support simulation with lookup table
    // adapted from https://github.com/solana-foundation/anchor/blob/0bdfa3f760635cc83bbda13f9a9d22d1558d1776/ts/packages/anchor/src/program/namespace/views.ts#L26C7-L42C39
    const ix = await program.methods
      .resolveExecuteVaaV1()
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
      (i) => i.name === "resolveExecuteVaaV1"
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
