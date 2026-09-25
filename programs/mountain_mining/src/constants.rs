use anchor_lang::prelude::*;

pub const DECIMALS: u8 = 9;
pub const TOTAL_SUPPLY_TOKENS: u64 = 1_000_000_000;
pub const TOTAL_SUPPLY_BASE_UNITS: u64 = 1_000_000_000_000_000_000;
pub const MINING_PERIOD_SECONDS: u64 = 630_720_000;
pub const TOTAL_PASS_COUNT: u32 = 100_000;
pub const TOTAL_POWER: u64 = 486_000;
pub const PHASE_COUNT: usize = 3;
pub const CLASS_COUNT: usize = 7;
pub const PHASE_CAPS: [u32; PHASE_COUNT] = [10_000, 10_000, 80_000];
pub const PROTOCOL_CONFIG_SEED: &[u8] = b"protocol_config";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
pub const MINING_STATE_SEED: &[u8] = b"mining_state";
pub const CUSTODY_SEED: &[u8] = b"custody";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassConfig {
    pub class_id: u8,
    pub name: &'static str,
    pub count: u32,
    pub multiplier: u64,
}

pub const CLASS_TABLE: [ClassConfig; CLASS_COUNT] = [
    ClassConfig {
        class_id: 0,
        name: "Stone",
        count: 40_000,
        multiplier: 1,
    },
    ClassConfig {
        class_id: 1,
        name: "Obsidian",
        count: 25_000,
        multiplier: 2,
    },
    ClassConfig {
        class_id: 2,
        name: "Iron",
        count: 15_000,
        multiplier: 4,
    },
    ClassConfig {
        class_id: 3,
        name: "Steel",
        count: 10_000,
        multiplier: 8,
    },
    ClassConfig {
        class_id: 4,
        name: "Titanium",
        count: 6_000,
        multiplier: 16,
    },
    ClassConfig {
        class_id: 5,
        name: "Diamond",
        count: 3_000,
        multiplier: 32,
    },
    ClassConfig {
        class_id: 6,
        name: "Mithril",
        count: 1_000,
        multiplier: 64,
    },
];

pub const _: () = {
    let mut count_sum: u32 = 0;
    let mut weighted_sum: u64 = 0;
    let mut i = 0;
    while i < CLASS_TABLE.len() {
        count_sum += CLASS_TABLE[i].count;
        weighted_sum += (CLASS_TABLE[i].count as u64) * CLASS_TABLE[i].multiplier;
        i += 1;
    }
    assert!(count_sum == TOTAL_PASS_COUNT);
    assert!(weighted_sum == TOTAL_POWER);
};

pub fn class_config(class_id: u8) -> Option<ClassConfig> {
    CLASS_TABLE
        .iter()
        .copied()
        .find(|entry| entry.class_id == class_id)
}
