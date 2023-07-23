import { Program } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { Counter } from "../target/types/counter";
import { sleep } from "@switchboard-xyz/common";

export const getGlobalPda = (program: Program<Counter>) => {
  const [globalPda, _globalPdaBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("global6")],
    program.programId
  );
  return globalPda;
};
export const getGlobalData = async (program: Program<Counter>) => {
  const [globalPda, _globalPdaBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("global6")],
    program.programId
  );
  const data = await program.account.global.fetch(globalPda);
  return data;
};

export async function printLogs(
  connection: Connection,
  tx: string,
  v0Txn?: boolean,
  delay = 3000
) {
  await sleep(delay);
  const parsed = await connection.getParsedTransaction(tx, {
    commitment: "confirmed",
    maxSupportedTransactionVersion: v0Txn ? 0 : undefined,
  });
  console.log(parsed?.meta?.logMessages?.join("\n"));
}

export async function handleFailedTxnLogs(
  connection: Connection,
  error: unknown
) {
  const errorString = `${error}`;
  const regex = /Raw transaction (\S+)/;
  const match = errorString.match(regex);
  const base58String = match ? match[1] : null;
  if (base58String) {
    console.log(base58String);
    await printLogs(connection, base58String);
  } else {
    console.log(`Failed to extract txn sig from: ${errorString}`);
  }

  throw error;
}
