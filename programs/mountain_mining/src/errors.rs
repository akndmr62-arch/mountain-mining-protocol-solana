use anchor_lang::prelude::*;

#[error_code]
pub enum MmpError {
    #[msg("Arithmetic overflow or underflow occurred.")]
    MathOverflow,
    #[msg("The requested mining phase is invalid.")]
    InvalidPhase,
    #[msg("The requested class identifier is invalid.")]
    InvalidClass,
    #[msg("The requested minting phase is currently paused.")]
    PhasePaused,
    #[msg("The requested minting phase has reached its cap.")]
    PhaseCapExceeded,
    #[msg("Collection NFT has already been created.")]
    CollectionAlreadyCreated,
    #[msg("Collection NFT has not been created yet.")]
    CollectionNotCreated,
    #[msg("Only the configured admin can perform this action.")]
    InvalidAdmin,
    #[msg("This Mining Pass is already mining.")]
    AlreadyMining,
    #[msg("This Mining Pass is not actively mining.")]
    NotMining,
    #[msg("The signer does not own the required NFT or mining position.")]
    InvalidOwner,
    #[msg("The provided timestamp is invalid.")]
    InvalidTimestamp,
    #[msg("All class capacity has been exhausted.")]
    CapacityExhausted,
    #[msg("The supplied metadata PDA does not match the expected Metaplex PDA.")]
    InvalidMetadataPda,
    #[msg("The supplied master edition PDA does not match the expected Metaplex PDA.")]
    InvalidEditionPda,
    #[msg("The supplied mint account does not match protocol configuration.")]
    InvalidMint,
    #[msg("The supplied PDA authority is invalid.")]
    InvalidAuthority,
    #[msg("Failed to read slot hashes sysvar data.")]
    InvalidSlotHashes,
    #[msg("Required PDA bump was not available.")]
    MissingBump,
    #[msg("The provided NFT mint is not a 1-of-1 Mining Pass NFT.")]
    InvalidPassMint,
    #[msg("The provided token account balance is invalid for the requested action.")]
    InvalidTokenBalance,
    #[msg("The NFT custody account is missing or malformed.")]
    InvalidCustody,
    #[msg("The MMP mint account has already been initialized.")]
    MintAlreadyInitialized,
    #[msg("The target token account must not already exist for this instruction.")]
    TokenAccountAlreadyInitialized,
}
