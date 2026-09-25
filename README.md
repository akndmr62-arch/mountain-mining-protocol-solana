# Mountain Mining Protocol (MMP)

Mountain Mining Protocol is a devnet-ready-only Solana/Anchor implementation of an NFT-gated passive mining protocol. A fixed 1,000,000,000 MMP supply (9 decimals, integer-only accounting) is emitted over a 20-year target lifecycle to 100,000 Mining Pass NFTs across seven immutable classes.

> **Status:** development / devnet-ready only. This repository is **not deployed to mainnet** and the bundled program keypair is a sandbox development key that **must be regenerated with `anchor keys sync` before any real deployment**.

## Architecture summary

- **On-chain program:** `/home/runner/work/mountain-mining-protocol-solana/mountain-mining-protocol-solana/programs/mountain_mining`
- **Framework:** Anchor + classic SPL Token (not Token-2022)
- **Metadata:** dependency-free Metaplex CPI builders in `/home/runner/work/mountain-mining-protocol-solana/mountain-mining-protocol-solana/programs/mountain_mining/src/metaplex.rs`
- **Tests:** Rust unit tests for reward math and protocol helpers, Anchor/Mocha integration tests under `/home/runner/work/mountain-mining-protocol-solana/mountain-mining-protocol-solana/tests`
- **Frontend:** Next.js + TypeScript skeleton in `/home/runner/work/mountain-mining-protocol-solana/mountain-mining-protocol-solana/frontend`

## Fixed economic constants

- **Total supply:** 1,000,000,000 MMP
- **Decimals:** 9
- **Raw base-unit cap:** 1,000,000,000,000,000,000
- **Mining period:** 630,720,000 seconds (20 years)
- **Total passes:** 100,000
- **Total weighted power:** 486,000
- **Distribution phases:** 10,000 airdrop / 10,000 early access / 80,000 public sale

Class table:

| Class | Count | Multiplier |
| --- | ---: | ---: |
| Stone | 40,000 | 1x |
| Obsidian | 25,000 | 2x |
| Iron | 15,000 | 4x |
| Steel | 10,000 | 8x |
| Titanium | 6,000 | 16x |
| Diamond | 3,000 | 32x |
| Mithril | 1,000 | 64x |

## Reward formula

```text
reward = floor(
    elapsed_seconds * class_multiplier * TOTAL_SUPPLY
    / (MINING_PERIOD_SECONDS * TOTAL_POWER)
)
```

- All reward math is integer-only.
- All intermediate arithmetic uses `u128` with checked operations.
- Claim-time minting applies `min(calculated_reward, remaining_supply)` so supply-cap exhaustion never reverts solely because theoretical rewards exceeded the remaining supply.

## Workspace layout

```text
Anchor.toml
Cargo.toml
programs/mountain_mining/
tests/
migrations/
frontend/
target/deploy/mountain_mining-keypair.json
```

## Build and test

### Rust / Anchor

```bash
cargo test -p mountain_mining
anchor build
anchor test
```

### Frontend

```bash
cd /home/runner/work/mountain-mining-protocol-solana/mountain-mining-protocol-solana/frontend
npm install
npm run dev
```

The frontend attempts to load `../target/idl/mountain_mining.json`. If that IDL is not present yet, Mine / Claim / Stop stay disabled with a visible “not wired” message instead of faking success.

## Operational notes

- `initialize_protocol` creates the immutable MMP mint with the mint-authority PDA as the only mint signer.
- `create_collection` and `mint_pass` are admin-triggered issuance flows for the Mining Pass collection only; they do **not** create any admin path to mint MMP or seize user assets.
- `start_mining` locks the NFT into a PDA-controlled custody ATA.
- `claim` mints accrued MMP to the owner ATA.
- `stop_mining` auto-settles first, then releases the NFT.
- Phase pausing is limited to Mining Pass issuance. There is **no** admin pause/kill-switch for user mining, claims, or withdrawals.

## Upgrade authority note

This repository does not perform any deployment. If deployed with Anchor on devnet, the program would still follow the normal Solana/Anchor upgrade-authority model. That upgrade authority decision must be made explicitly at deployment time; this repository does not claim immutability or a mainnet deployment.

## Known limitations

1. **Randomness is dev-grade only.** Class assignment uses `SlotHashes`-based entropy and is **not** production-grade VRF. Replace it with Switchboard/ORAO VRF before any mainnet consideration.
2. **Program keypair is sandbox-only.** `/home/runner/work/mountain-mining-protocol-solana/mountain-mining-protocol-solana/target/deploy/mountain_mining-keypair.json` exists only to make local development/builds deterministic and must be replaced via `anchor keys sync` before any real deployment.
3. **Frontend wiring depends on a built IDL.** The action pages intentionally degrade to disabled state when `target/idl/mountain_mining.json` has not been generated yet.
4. **Metaplex CPI layouts are hand-maintained.** If the Metaplex Token Metadata program changes instruction discriminators or Borsh layouts, `/programs/mountain_mining/src/metaplex.rs` must be updated accordingly.
