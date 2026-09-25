import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
} from "@solana/web3.js";
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore - generated after anchor build
import { MountainMining } from "../target/types/mountain_mining";

const METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
const PROGRAM_SEEDS = {
  protocolConfig: Buffer.from("protocol_config"),
  mintAuthority: Buffer.from("mint_authority"),
  miningState: Buffer.from("mining_state"),
  custody: Buffer.from("custody"),
};

type MintedPass = {
  passMint: Keypair;
  miningState: PublicKey;
  owner: Keypair;
  ownerPassToken: PublicKey;
};

describe("mountain-mining", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MountainMining as Program<MountainMining>;
  const admin = provider.wallet as anchor.Wallet;
  const protocolConfig = PublicKey.findProgramAddressSync(
    [PROGRAM_SEEDS.protocolConfig],
    program.programId,
  )[0];
  const mintAuthority = PublicKey.findProgramAddressSync(
    [PROGRAM_SEEDS.mintAuthority],
    program.programId,
  )[0];

  const mmpMint = Keypair.generate();
  const collectionMint = Keypair.generate();
  const collectionMetadata = metadataPda(collectionMint.publicKey);
  const collectionEdition = masterEditionPda(collectionMint.publicKey);
  const collectionToken = getAssociatedTokenAddressSync(collectionMint.publicKey, admin.publicKey);
  const mintedPasses: MintedPass[] = [];

  before(async () => {
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(admin.publicKey, 5_000_000_000),
      "confirmed",
    );
  });

  it("initializes protocol", async () => {
    await program.methods
      .initializeProtocol()
      .accounts({
        protocolConfig,
        admin: admin.publicKey,
        mmpMint: mmpMint.publicKey,
        mintAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([mmpMint])
      .rpc();

    const config = await program.account.protocolConfig.fetch(protocolConfig);
    expect(config.totalMinted.toString()).to.equal("0");
    expect(config.remainingSupply.toString()).to.equal("1000000000000000000");
  });

  it("creates the collection NFT", async () => {
    await program.methods
      .createCollection({
        name: "Mountain Mining Pass Collection",
        symbol: "MMPASS",
        uri: "https://example.com/collection.json",
      })
      .accounts({
        protocolConfig,
        admin: admin.publicKey,
        mintAuthority,
        collectionMint: collectionMint.publicKey,
        collectionMetadata,
        collectionMasterEdition: collectionEdition,
        collectionTokenAccount: collectionToken,
        tokenMetadataProgram: METADATA_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([collectionMint])
      .rpc();

    const config = await program.account.protocolConfig.fetch(protocolConfig);
    expect(config.collectionMint.toBase58()).to.equal(collectionMint.publicKey.toBase58());
  });

  for (const phase of [0, 1, 2]) {
    it(`mints a pass in phase ${phase}`, async () => {
      const owner = Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(owner.publicKey, 3_000_000_000),
        "confirmed",
      );

      const passMint = Keypair.generate();
      const passMetadata = metadataPda(passMint.publicKey);
      const passEdition = masterEditionPda(passMint.publicKey);
      const recipientPassToken = getAssociatedTokenAddressSync(passMint.publicKey, owner.publicKey);
      const miningState = PublicKey.findProgramAddressSync(
        [PROGRAM_SEEDS.miningState, passMint.publicKey.toBuffer()],
        program.programId,
      )[0];
      const custodyAuthority = PublicKey.findProgramAddressSync(
        [PROGRAM_SEEDS.custody, passMint.publicKey.toBuffer()],
        program.programId,
      )[0];

      await program.methods
        .mintPass(phase, {
          name: `Mining Pass #${phase + 1}`,
          symbol: "MMPASS",
          uri: `https://example.com/pass-${phase + 1}.json`,
        })
        .accounts({
          protocolConfig,
          admin: admin.publicKey,
          mintAuthority,
          passMint: passMint.publicKey,
          passMetadata,
          passMasterEdition: passEdition,
          recipient: owner.publicKey,
          recipientPassToken,
          miningState,
          custodyAuthority,
          slotHashes: SYSVAR_SLOT_HASHES_PUBKEY,
          collectionMint: collectionMint.publicKey,
          collectionMetadata,
          collectionMasterEdition: collectionEdition,
          tokenMetadataProgram: METADATA_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([passMint])
        .rpc();

      const config = await program.account.protocolConfig.fetch(protocolConfig);
      expect(config.phaseMinted[phase]).to.equal(1);
      expect(config.classRemaining.reduce((acc, next) => acc + next, 0)).to.equal(99_999 - phase);

      const minedState = await program.account.miningState.fetch(miningState);
      expect(minedState.isMining).to.equal(false);
      mintedPasses.push({ passMint, miningState, owner, ownerPassToken: recipientPassToken });
    });
  }

  it("starts mining and locks the NFT in custody", async () => {
    const pass = mintedPasses[0];
    const custodyAuthority = PublicKey.findProgramAddressSync(
      [PROGRAM_SEEDS.custody, pass.passMint.publicKey.toBuffer()],
      program.programId,
    )[0];
    const custodyPassToken = getAssociatedTokenAddressSync(pass.passMint.publicKey, custodyAuthority, true);

    await program.methods
      .startMining()
      .accounts({
        owner: pass.owner.publicKey,
        passMint: pass.passMint.publicKey,
        miningState: pass.miningState,
        ownerPassToken: pass.ownerPassToken,
        custodyAuthority,
        custodyPassToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([pass.owner])
      .rpc();

    const ownerAccount = await getAccount(provider.connection, pass.ownerPassToken);
    const custodyAccount = await getAccount(provider.connection, custodyPassToken);
    expect(Number(ownerAccount.amount)).to.equal(0);
    expect(Number(custodyAccount.amount)).to.equal(1);
  });

  it("rejects double mining and non-owner claim attempts", async () => {
    const pass = mintedPasses[0];
    const custodyAuthority = PublicKey.findProgramAddressSync(
      [PROGRAM_SEEDS.custody, pass.passMint.publicKey.toBuffer()],
      program.programId,
    )[0];
    const custodyPassToken = getAssociatedTokenAddressSync(pass.passMint.publicKey, custodyAuthority, true);
    await expect(
      program.methods
        .startMining()
        .accounts({
          owner: pass.owner.publicKey,
          passMint: pass.passMint.publicKey,
          miningState: pass.miningState,
          ownerPassToken: pass.ownerPassToken,
          custodyAuthority,
          custodyPassToken,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([pass.owner])
        .rpc(),
    ).to.be.rejected;

    const stranger = Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(stranger.publicKey, 1_000_000_000),
      "confirmed",
    );
    const ownerMmpToken = getAssociatedTokenAddressSync(mmpMint.publicKey, stranger.publicKey);
    await expect(
      program.methods
        .claim()
        .accounts({
          protocolConfig,
          owner: stranger.publicKey,
          mmpMint: mmpMint.publicKey,
          passMint: pass.passMint.publicKey,
          miningState: pass.miningState,
          mintAuthority,
          ownerMmpToken,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([stranger])
        .rpc(),
    ).to.be.rejected;
  });

  it("claims rewards repeatedly", async () => {
    const pass = mintedPasses[0];
    await sleep(1500);
    const ownerMmpToken = getAssociatedTokenAddressSync(mmpMint.publicKey, pass.owner.publicKey);

    await program.methods
      .claim()
      .accounts({
        protocolConfig,
        owner: pass.owner.publicKey,
        mmpMint: mmpMint.publicKey,
        passMint: pass.passMint.publicKey,
        miningState: pass.miningState,
        mintAuthority,
        ownerMmpToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([pass.owner])
      .rpc();

    const firstBalance = Number((await getAccount(provider.connection, ownerMmpToken)).amount);
    expect(firstBalance).to.be.greaterThan(0);

    await sleep(1200);
    await program.methods
      .claim()
      .accounts({
        protocolConfig,
        owner: pass.owner.publicKey,
        mmpMint: mmpMint.publicKey,
        passMint: pass.passMint.publicKey,
        miningState: pass.miningState,
        mintAuthority,
        ownerMmpToken,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([pass.owner])
      .rpc();

    const secondBalance = Number((await getAccount(provider.connection, ownerMmpToken)).amount);
    expect(secondBalance).to.be.greaterThan(firstBalance);
  });

  it("cannot claim while not mining and stop auto-settles + releases", async () => {
    const passivePass = mintedPasses[1];
    const passiveOwnerMmpToken = getAssociatedTokenAddressSync(mmpMint.publicKey, passivePass.owner.publicKey);
    await expect(
      program.methods
        .claim()
        .accounts({
          protocolConfig,
          owner: passivePass.owner.publicKey,
          mmpMint: mmpMint.publicKey,
          passMint: passivePass.passMint.publicKey,
          miningState: passivePass.miningState,
          mintAuthority,
          ownerMmpToken: passiveOwnerMmpToken,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([passivePass.owner])
        .rpc(),
    ).to.be.rejected;

    const activePass = mintedPasses[0];
    const ownerMmpToken = getAssociatedTokenAddressSync(mmpMint.publicKey, activePass.owner.publicKey);
    const custodyAuthority = PublicKey.findProgramAddressSync(
      [PROGRAM_SEEDS.custody, activePass.passMint.publicKey.toBuffer()],
      program.programId,
    )[0];
    const custodyPassToken = getAssociatedTokenAddressSync(activePass.passMint.publicKey, custodyAuthority, true);
    const ownerPassAta = getAssociatedTokenAddressSync(activePass.passMint.publicKey, activePass.owner.publicKey);
    const preStopBalance = Number((await getAccount(provider.connection, ownerMmpToken)).amount);
    await sleep(1200);

    await program.methods
      .stopMining()
      .accounts({
        protocolConfig,
        owner: activePass.owner.publicKey,
        mmpMint: mmpMint.publicKey,
        passMint: activePass.passMint.publicKey,
        miningState: activePass.miningState,
        mintAuthority,
        ownerMmpToken,
        custodyAuthority,
        custodyPassToken,
        ownerPassToken: ownerPassAta,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([activePass.owner])
      .rpc();

    const postStopBalance = Number((await getAccount(provider.connection, ownerMmpToken)).amount);
    const releasedPassBalance = Number((await getAccount(provider.connection, ownerPassAta)).amount);
    expect(postStopBalance).to.be.greaterThan(preStopBalance);
    expect(releasedPassBalance).to.equal(1);
  });
});

function metadataPda(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), METADATA_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    METADATA_PROGRAM_ID,
  )[0];
}

function masterEditionPda(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), METADATA_PROGRAM_ID.toBuffer(), mint.toBuffer(), Buffer.from("edition")],
    METADATA_PROGRAM_ID,
  )[0];
}

async function sleep(ms: number): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, ms));
}
