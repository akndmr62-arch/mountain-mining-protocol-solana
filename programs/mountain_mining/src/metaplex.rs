use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

/// Metaplex Token Metadata program ID.
///
/// The protocol builds the minimal CPI instructions it needs directly instead of depending on the
/// `mpl-token-metadata` crate, because that crate frequently pins a different `solana-program`
/// version than Anchor-based programs. Keep the discriminators and Borsh layouts in sync with the
/// upstream Metaplex program whenever upgrading.
pub const TOKEN_METADATA_PROGRAM_ID: Pubkey =
    pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Creator {
    pub address: Pubkey,
    pub verified: bool,
    pub share: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Collection {
    pub verified: bool,
    pub key: Pubkey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct DataV2 {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<Creator>>,
    pub collection: Option<Collection>,
    pub uses: Option<Uses>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Uses {
    pub use_method: u8,
    pub remaining: u64,
    pub total: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct CreateMetadataAccountV3Args {
    pub data: DataV2,
    pub is_mutable: bool,
    pub collection_details: Option<CollectionDetails>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum CollectionDetails {
    V1 { size: u64 },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct CreateMasterEditionV3Args {
    pub max_supply: Option<u64>,
}

pub fn metadata_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"metadata",
            TOKEN_METADATA_PROGRAM_ID.as_ref(),
            mint.as_ref(),
        ],
        &TOKEN_METADATA_PROGRAM_ID,
    )
    .0
}

pub fn master_edition_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"metadata",
            TOKEN_METADATA_PROGRAM_ID.as_ref(),
            mint.as_ref(),
            b"edition",
        ],
        &TOKEN_METADATA_PROGRAM_ID,
    )
    .0
}

pub fn create_metadata_account_v3(
    metadata: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    args: CreateMetadataAccountV3Args,
) -> Instruction {
    let mut data = vec![33u8];
    data.extend(args.try_to_vec().expect("serialize create metadata"));

    Instruction {
        program_id: TOKEN_METADATA_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(metadata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(update_authority, true),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::sysvar::rent::ID, false),
        ],
        data,
    }
}

pub fn create_master_edition_v3(
    edition: Pubkey,
    mint: Pubkey,
    update_authority: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    metadata: Pubkey,
    max_supply: Option<u64>,
) -> Instruction {
    let mut data = vec![17u8];
    data.extend(
        CreateMasterEditionV3Args { max_supply }
            .try_to_vec()
            .expect("serialize master edition"),
    );

    Instruction {
        program_id: TOKEN_METADATA_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(edition, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(update_authority, true),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new(payer, true),
            AccountMeta::new(metadata, false),
            AccountMeta::new_readonly(anchor_spl::token::ID, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::system_program::ID, false),
            AccountMeta::new_readonly(anchor_lang::solana_program::sysvar::rent::ID, false),
        ],
        data,
    }
}

pub fn set_and_verify_collection(
    metadata: Pubkey,
    collection_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    collection_mint: Pubkey,
    collection_metadata: Pubkey,
    collection_master_edition: Pubkey,
) -> Instruction {
    Instruction {
        program_id: TOKEN_METADATA_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(metadata, false),
            AccountMeta::new(collection_authority, true),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(update_authority, false),
            AccountMeta::new_readonly(collection_mint, false),
            AccountMeta::new_readonly(collection_metadata, false),
            AccountMeta::new_readonly(collection_master_edition, false),
        ],
        data: vec![25u8],
    }
}
