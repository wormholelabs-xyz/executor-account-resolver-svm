import { AccountMeta } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolanaAccountResolver } from "../target/types/solana_account_resolver";

describe("solana-account-resolver", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SolanaAccountResolver as Program<SolanaAccountResolver>;

  const payer = anchor.web3.Keypair.generate();
  // airdrop to payer
  it("Is initialized!", async () => {
    await anchor.getProvider().connection.confirmTransaction(
      await anchor.getProvider().connection.requestAirdrop(payer.publicKey, 1000000000),
      "confirmed"
    );

    // const [bar, _] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("bar"), Buffer.from([10])], program.programId);
    // const [bar, _] = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("bar"), Buffer.from([10])], program.programId);
    const tx = await program.methods.initialize().accounts(
      {
        payer: program.provider.publicKey,
        // foo: fooKeypair.publicKey,
        // bar: bar
      }
    ).rpc();
    console.log("Your transaction signature", tx);
    try {
      const result = await resolveInstructions(program);
      for (const group of result) {
        // TODO: send whole group as tx etc.
        for (const instruction of group) {
          console.log(instruction);

          // 32 9s
          let payerConst = new anchor.web3.PublicKey(Array(32).fill(9));

          let tx = new anchor.web3.Transaction();
          let accountsWithPayerOverride = instruction.accounts.map((account) => {
            if (account.pubkey.equals(payerConst)) {
              return { ...account, pubkey: payer.publicKey };
            }
            return account;
          });
          tx.add(new anchor.web3.TransactionInstruction({
            keys: accountsWithPayerOverride,
            programId: instruction.programId,
            data: instruction.data,
          }));
          let signature = await anchor.getProvider().connection.sendTransaction(tx, [payer]);
          await anchor.getProvider().connection.confirmTransaction(signature, "confirmed");
        }
      }
    } catch (e) {
      console.log("Error", e);
    }
  });
});

type Instruction = {
  programId: anchor.web3.PublicKey;
  accounts: AccountMeta[],
  data: Buffer,
}


// a function that calls accountsToExecute repeatedly until it returns ok. as
// long as it returns missing, we add the returned missing keys to
// remainingAccounts and call accountsToExecute again
async function resolveInstructions(program: Program<SolanaAccountResolver>): Promise<Instruction[][]> {
  let remainingAccounts: AccountMeta[] = [];
  let runs = 0;
  while (true) {
    runs++;
    const result = await program.methods.accountsToExecute().remainingAccounts(remainingAccounts).view();
    if (result.resolved) {
      console.log("Runs", runs);
      return result.resolved[0][0];
    } else {
      let newAccountMetas = result.missing[0].map((key) => { return { pubkey: key, isSigner: false, isWritable: false } });
      remainingAccounts.push(...newAccountMetas);
    }
  }
}
