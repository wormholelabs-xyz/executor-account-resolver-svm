import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ExampleShimVaaVerification } from "../target/types/example_shim_vaa_verification";
import { resolveInstructions } from "./utils";
import { expect } from "chai";
import type { WormholeVerifyVaaShim } from "../idls/wormhole_verify_vaa_shim";
import WormholeVerifyVaaShimIdl from "../idls/wormhole_verify_vaa_shim.json";

const VAA = Buffer.from(
  "01000000040f01ae82ab39c8404daf3e9a67894ae86ca03a52479cc6da3cba32db96df4e2907fa75015583a8171ff22dbe1dd411a83332f3cedc9bd6244a9a9a3fa31543e5e115000300d9f32967e1a1a24bfea28fe34a490e666c9e5c8db1f227de551111a94acca82f2569b886dbdf5766cda897ae47738869e33c3fce8b7b6c879c396832a827d900048761b197f54b8af349e3ad19ddb517de1818ca9f97a29f6be7743bdffa5c114b2e2660158ef38b7d1bd3625faed43fb691dc0309c7103fd810208133e0ba16600005635a158e5f0be25c70396a8ab4bee57f3a650d6d8577f2f63f0316af94fd9e7944770550daa4174108878588203440ce88094724dc378341a64666103779427001066ee90a7bc4570583697f02c84fcecd33d6c6bdf13897afa1e9c5d801bf271e3a2d28f3d64627d78de248f9a17ca2d10deb185cdd7969f206ce6899b3738b57840107468d907190a94f99e97dfef5374f0e9e76d2f36e54eecaa3a971a571bf405fe576bc6fff1c17afde75c6c0e69fac5011faba914c0287dfd9034c60d7f9f78e2e010879f665fcb10104d2ec54936e7e5fddee08f0a4dbe791354255677c702332fb6624abcde35310649be2f7ae615a766f72576a766164c73f92aaa3e2ea2d0643af0009789d2776289261b0469ced4c420e1d2b69c7f7ee70c94771140ec1f51965b42b0c90d4bac5368e175ad8ced73a064d639edf2eba330ead81bb7f5a2287448dc6000ab18adc47d489162954d6ca485b2ada5dd942c969b7d858f2ebd4e5c8e50639bc3b8c342a306a6efc529e597618760b208fcbee8804bb55d29be4f4f497ba1cdf010c62b2a95b96624c65a029cb77aa1c172f948f0da5a614d4299bfe254eacf72ed84d45e7b3980f9b4f9c4a3410545264a0ec9a12702c8cbed449f60271d40be0d7010d0d5ced17abb08bc98701ee753cda572f8534b00cf692b2ddcf6a9c02d61e5eb91dd66b03b385de3ccf8c387d322a2e87dc0ab0d85b002c3c06bc07a6b44a5007010e6e1e5dd6563cb939efed4c280e4c6223fea70db8ca39218a2e87d462425592d7066114eda80303603e3e4d384e071003a8fc91fd2246aa00167f20bf09e2c6e20110bfe9d0117aecb762182ee367de315a3072912e868f2651c715fb26ce36c142f17d02735c49a69666ca3be42eb3faeedcf1e5217ab8e5ed14eca74360e75c81ea001143313da4d640ffc178afb568829c95341de56470390c182460078b69e86791ae12c1c1ae8d3e925960c42578787aabfc75fa7548943fb882dae0cfad6cac37dd01121aea952af224b2b02557d8245c7c5e12504f13bfb55647b93c3f32b0243ecbb06697a0c11189e696b825c69dbb5bd53fefd62f145c4dacd3dd74ecd61f038e1f0000000000b105ad5e000100000000000000000000000000000000000000000000000000000000000000046ffbd35fa67c5cfc20000000000000000000000000000000000000000000546f6b656e4272696467650100000033289e998e357c96dbfd8490b853595e0d48639ede2d1aed4a819edcc00165904c",
  "hex"
);
const sigStart = 6;
const numSigners = VAA[5];
const sigLength = 66;
const vaaBody = VAA.subarray(sigStart + sigLength * numSigners);
const guardianSetIndex = VAA.subarray(1, 5);
const guardianSet = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("GuardianSet"), guardianSetIndex],
  new anchor.web3.PublicKey("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth")
)[0];

// from https://github.com/wormhole-foundation/wormhole/blob/b6aee18398aa4fd915f7ecf2149b960b71ae3463/sdk/js/src/vaa/wormhole.ts#L26
export function parseVaa(vaa: Buffer) {
  const signedVaa = Buffer.isBuffer(vaa) ? vaa : Buffer.from(vaa as Uint8Array);
  const sigStart = 6;
  const numSigners = signedVaa[5];
  const sigLength = 66;

  const guardianSignatures = [];
  for (let i = 0; i < numSigners; ++i) {
    const start = sigStart + i * sigLength;
    guardianSignatures.push({
      index: signedVaa[start],
      signature: signedVaa.subarray(start + 1, start + 66),
    });
  }

  const body = signedVaa.subarray(sigStart + sigLength * numSigners);

  return {
    version: signedVaa[0],
    guardianSetIndex: signedVaa.readUInt32BE(1),
    guardianSignatures,
    timestamp: body.readUInt32BE(0),
    nonce: body.readUInt32BE(4),
    emitterChain: body.readUInt16BE(8),
    emitterAddress: body.subarray(10, 42),
    sequence: body.readBigUInt64BE(42),
    consistencyLevel: body[50],
    payload: body.subarray(51),
  };
}

describe("example-shim-vaa-verification", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .ExampleShimVaaVerification as Program<ExampleShimVaaVerification>;

  it("Is verifies a VAA via the shim!", async () => {
    const payer = program.provider.wallet?.payer;
    if (!payer) throw new Error("Expected payer");
    const result = await resolveInstructions(
      program.provider,
      program.programId,
      payer,
      vaaBody,
      guardianSet
    );
    expect(result.length).to.eq(1);
    const group = result[0];
    expect(group.instructions.length).to.eq(1);
    const instruction = group.instructions[0];
    let payerConst = new anchor.web3.PublicKey(
      Buffer.from("payer_00000000000000000000000000")
    );
    let gsConst = new anchor.web3.PublicKey(
      Buffer.from("guardian_set_0000000000000000000")
    );
    let sigConst = new anchor.web3.PublicKey(
      Buffer.from("shim_vaa_sigs_000000000000000000")
    );
    expect(instruction.accounts.length).to.eq(3);
    expect(instruction.accounts[0].pubkey.toString()).to.eq(
      guardianSet.toString()
    );
    expect(instruction.accounts[1].pubkey.toString()).to.eq(
      sigConst.toString()
    );
    expect(instruction.accounts[2].pubkey.toString()).to.eq(
      "EFaNWErqAtVWufdNb7yofSHHfWFos843DFpu4JBw24at"
    );
    console.log(instruction);

    const shimProgram = new Program<WormholeVerifyVaaShim>(
      WormholeVerifyVaaShimIdl as WormholeVerifyVaaShim,
      program.provider
    );
    const signatureKeypair = anchor.web3.Keypair.generate();
    const vaa = parseVaa(VAA);

    const postSigTx = await shimProgram.methods
      .postSignatures(
        vaa.guardianSetIndex,
        vaa.guardianSignatures.length,
        vaa.guardianSignatures
          .slice(0, 13)
          .map((s) => [s.index, ...s.signature])
      )
      .accounts({ guardianSignatures: signatureKeypair.publicKey })
      .signers([signatureKeypair])
      .rpc();
    console.log("postedSigs:", postSigTx);

    let accountsWithOverrides = instruction.accounts.map((account) => {
      if (account.pubkey.equals(payerConst)) {
        return { ...account, pubkey: payer.publicKey };
      }
      if (account.pubkey.equals(gsConst)) {
        return { ...account, pubkey: guardianSet };
      }
      if (account.pubkey.equals(sigConst)) {
        return { ...account, pubkey: signatureKeypair.publicKey };
      }
      return account;
    });
    const ix = new anchor.web3.TransactionInstruction({
      keys: accountsWithOverrides,
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
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: 420_000,
        }),
        ix,
        await shimProgram.methods
          .closeSignatures()
          .accounts({ guardianSignatures: signatureKeypair.publicKey })
          .instruction(),
      ],
      recentBlockhash: blockhash,
    }).compileToV0Message(luts);
    const tx = new anchor.web3.VersionedTransaction(messageV0);
    tx.sign([payer]);
    let signature = await program.provider.connection.sendTransaction(tx);
    const res = await program.provider.connection.confirmTransaction(
      { signature, blockhash, lastValidBlockHeight },
      "confirmed"
    );
    console.log("verified:", signature, res);
  });
});
