import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { splDiscriminate } from "@solana/spl-type-length-value";
import { expect } from "chai";
import { ExecutorAccountResolverSvmProgram } from "../target/types/executor_account_resolver_svm_program";

describe("executor-account-resolver", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .ExecutorAccountResolverSvmProgram as Program<ExecutorAccountResolverSvmProgram>;

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
