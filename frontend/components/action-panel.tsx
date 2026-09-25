"use client";

import { FormEvent, useState } from "react";
import { AnchorProvider, Program, type AnchorWallet, type Idl } from "@coral-xyz/anchor";
import { getAssociatedTokenAddressSync, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey, SystemProgram } from "@solana/web3.js";

type ActionMethod = "startMining" | "claim" | "stopMining";

export function ActionPanel({
  title,
  method,
  wired,
  reason,
}: {
  title: string;
  method: ActionMethod;
  wired: boolean;
  reason: string;
}) {
  const { connection } = useConnection();
  const wallet = useWallet();
  const [passMint, setPassMint] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!wired) {
      setStatus(reason);
      return;
    }
    if (!wallet.publicKey || !wallet.signTransaction) {
      setStatus("Connect a Phantom-compatible wallet first.");
      return;
    }

    try {
      setSubmitting(true);
      setStatus("Loading Anchor IDL and preparing transaction...");
      const response = await fetch("/api/idl");
      if (!response.ok) {
        throw new Error(`IDL fetch failed (${response.status}).`);
      }
      const idl = (await response.json()) as Idl & { address?: string; metadata?: { address?: string } };
      const programId = new PublicKey(idl.address ?? idl.metadata?.address ?? "");
      const provider = new AnchorProvider(connection, wallet as AnchorWallet, {});
      const program = new Program(idl, programId, provider);
      const owner = wallet.publicKey;
      const passMintKey = new PublicKey(passMint.trim());
      const protocolConfig = PublicKey.findProgramAddressSync([Buffer.from("protocol_config")], program.programId)[0];
      const mintAuthority = PublicKey.findProgramAddressSync([Buffer.from("mint_authority")], program.programId)[0];
      const miningState = PublicKey.findProgramAddressSync(
        [Buffer.from("mining_state"), passMintKey.toBuffer()],
        program.programId,
      )[0];
      const custodyAuthority = PublicKey.findProgramAddressSync(
        [Buffer.from("custody"), passMintKey.toBuffer()],
        program.programId,
      )[0];
      const ownerPassToken = getAssociatedTokenAddressSync(passMintKey, owner);
      const custodyPassToken = getAssociatedTokenAddressSync(passMintKey, custodyAuthority, true);

      if (method === "startMining") {
        const signature = await program.methods.startMining().accounts({
          owner,
          passMint: passMintKey,
          miningState,
          ownerPassToken,
          custodyAuthority,
          custodyPassToken,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        }).rpc();
        setStatus(`Mine transaction sent: ${signature}`);
        return;
      }

      const config = (await program.account.protocolConfig.fetch(protocolConfig)) as { mmpMint: PublicKey };
      const mmpMint = new PublicKey(config.mmpMint);
      const ownerMmpToken = getAssociatedTokenAddressSync(mmpMint, owner);

      if (method === "claim") {
        const signature = await program.methods.claim().accounts({
          protocolConfig,
          owner,
          mmpMint,
          passMint: passMintKey,
          miningState,
          mintAuthority,
          ownerMmpToken,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        }).rpc();
        setStatus(`Claim transaction sent: ${signature}`);
        return;
      }

      const signature = await program.methods.stopMining().accounts({
        protocolConfig,
        owner,
        mmpMint,
        passMint: passMintKey,
        miningState,
        mintAuthority,
        ownerMmpToken,
        custodyAuthority,
        custodyPassToken,
        ownerPassToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      }).rpc();
      setStatus(`Stop transaction sent: ${signature}`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="card">
      <h1>{title}</h1>
      <p className="muted">Enter a Mining Pass mint address and submit with a connected Phantom wallet.</p>
      <form onSubmit={onSubmit} className="grid">
        <label>
          <span className="muted">Mining Pass mint</span>
          <input
            className="input"
            value={passMint}
            onChange={(event) => setPassMint(event.target.value)}
            placeholder="Enter the NFT mint public key"
          />
        </label>
        <button className="button" type="submit" disabled={!wired || submitting || !passMint.trim()}>
          {submitting ? "Submitting..." : title}
        </button>
      </form>
      <div className="status">
        <strong>{wired ? "Wired" : "Not wired"}:</strong> {reason}
      </div>
      {status ? <div className="status">{status}</div> : null}
    </div>
  );
}
