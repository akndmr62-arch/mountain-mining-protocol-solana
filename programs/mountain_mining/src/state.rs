use crate::constants::{CLASS_COUNT, PHASE_COUNT};
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ProtocolConfig {
    pub bump: u8,
    pub mint_authority_bump: u8,
    pub admin: Pubkey,
    pub mmp_mint: Pubkey,
    pub collection_mint: Pubkey,
    pub total_supply: u64,
    pub total_minted: u64,
    pub remaining_supply: u64,
    pub phase_caps: [u32; PHASE_COUNT],
    pub phase_minted: [u32; PHASE_COUNT],
    pub phase_paused: [bool; PHASE_COUNT],
    pub class_remaining: [u32; CLASS_COUNT],
}

#[account]
#[derive(InitSpace)]
pub struct MiningState {
    pub state_bump: u8,
    pub custody_bump: u8,
    pub pass_mint: Pubkey,
    pub owner: Pubkey,
    pub class_id: u8,
    pub is_mining: bool,
    pub mining_start_ts: i64,
    pub last_claim_ts: i64,
    pub lifetime_claimed: u64,
}
