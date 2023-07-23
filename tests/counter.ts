import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Counter } from "../target/types/counter";
import { Keypair, PublicKey } from "@solana/web3.js";
import {
  AttestationQueueAccount,
  type BootstrappedAttestationQueue,
  NativeMint,
  parseMrEnclave,
  SwitchboardProgram,
  FunctionAccount,
  SwitchboardWallet,
  FunctionRequestAccount,
} from "@switchboard-xyz/solana.js";
import {
  getGlobalData,
  getGlobalPda,
  handleFailedTxnLogs,
  printLogs,
} from "./utils";
import { assert } from "chai";
import { toUtf8, sleep } from "@switchboard-xyz/common";

const MRENCLAVE = parseMrEnclave(
  Buffer.from("Y6keo0uTCiWDNcWwGjZ2jfTd4VFhrr6LC/6Mk1aiNCA=", "base64")
);
describe("counter", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Counter as Program<Counter>;

  const payer = (program.provider as anchor.AnchorProvider).publicKey;

  let switchboard: BootstrappedAttestationQueue;
  let functionAccount: FunctionAccount;
  let requestAccount: FunctionRequestAccount;
  let wallet: SwitchboardWallet;

  const [userPubkey, userBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("CUSTOM_RANDOMNESS"), payer.toBytes()],
    program.programId
  );
  const userTokenWallet = anchor.utils.token.associatedAddress({
    mint: NativeMint.address,
    owner: userPubkey,
  });

  before(async () => {
    const switchboardProgram = await SwitchboardProgram.fromProvider(
      program.provider as anchor.AnchorProvider
    );

    switchboard = await AttestationQueueAccount.bootstrapNewQueue(
      switchboardProgram
    );
    [functionAccount] =
      await switchboard.attestationQueue.account.createFunction({
        name: "test function",
        metadata: "this function handles XYZ for my protocol",
        schedule: "10 * * * * *",
        container: "switchboardlabs/basic-oracle-function",
        version: "latest",
        mrEnclave: MRENCLAVE,
        // authority: programStatePubkey,
      });
    wallet = await functionAccount.wallet;

    console.log(
      `state: ${switchboardProgram.attestationProgramState.publicKey}`
    );
    console.log(`attestationQueue: ${switchboard.attestationQueue.publicKey}`);
    console.log(`function: ${functionAccount.publicKey}`);
  });
  console.log("TEST 1");
  const global = getGlobalPda(program);
  console.log("TEST 2");

  it("Is initialized!", async () => {
    try {
      await program.methods
        .initialize()
        .accounts({
          global: global,
          initializer: payer,
          user: userPubkey,
          userTokenWallet,
          mint: NativeMint.address,
        })
        .rpc();
    } catch (error) {
      console.log("ERROR", error);
    }

    console.log("TEST 3");
    const data = await getGlobalData(program);
    console.log("Data", +data.count);
  });

  it("user_guess", async () => {
    const requestKeypair = Keypair.generate();

    requestAccount = new FunctionRequestAccount(
      switchboard.program,
      requestKeypair.publicKey
    );

    // try {
    try {
      const tx = await program.methods
        .addOne()
        .accounts({
          global: global,
          user: userPubkey,
          request: requestKeypair.publicKey,
          function: functionAccount.publicKey,
          requestEscrow: anchor.utils.token.associatedAddress({
            mint: NativeMint.address,
            owner: requestKeypair.publicKey,
          }),
          mint: NativeMint.address,
          state: switchboard.program.attestationProgramState.publicKey,
          attestationQueue: switchboard.attestationQueue.publicKey,
          switchboard: switchboard.program.attestationProgramId,
          userTokenWallet,
          initializer: payer,
        })
        .signers([requestKeypair])
        .rpc();
      console.log("user_guess transaction signature", tx);
      await printLogs(program.provider.connection, tx);
    } catch (error) {
      console.log("ERROR", error);
      await handleFailedTxnLogs(switchboard.program.connection, error);
    }

    const globalData1 = await program.account.global.fetch(global);
    console.log("globalData", +globalData1.count);
    const requestState = await requestAccount.loadData();
    const expectedRequestParams = `PID=${
      program.programId
    },MAX_GUESS=${255},USER=${userPubkey}`;
    const requestParams = toUtf8(requestState.containerParams);
    await sleep(12_000);
    const globalData2 = await program.account.global.fetch(global);
    const userData = await program.account.userState.fetch(userPubkey);
    console.log("globalData", +globalData2.count);
    console.log("userData", userData.currentRound);
    console.log("userData", userData.lastRound);
    assert(requestParams === expectedRequestParams, "Request params mismatch");
  });
});
