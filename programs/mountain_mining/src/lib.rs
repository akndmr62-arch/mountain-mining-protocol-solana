use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    hash::hashv,
    program::{invoke, invoke_signed},
    program_pack::Pack,
    system_instruction,
    sysvar::slot_hashes::SlotHashes,
};
use anchor_spl::{
    associated_token::{self, AssociatedToken},
    token::{
        self,
        spl_token::{self, instruction::AuthorityType, state::Mint as SplMint},
        Mint, MintTo, SetAuthority, Token, TokenAccount, Transfer,
    },
};

pub mod constants;
pub mod errors;
pub mod metaplex;
pub mod reward_math;
pub mod state;

use constants::*;
use errors::MmpError;
use metaplex::{Collection, CreateMetadataAccountV3Args, DataV2};
use reward_math::{calculate_reward, cap_reward_to_remaining_supply, RewardMathError};
use state::{MiningState, ProtocolConfig};

declare_id!("8VVeezk1LbGGqMMevb2WtisLgqQHvcQ6EYy5ozLgJkDX");

#[program]
pub mod mountain_mining {
    use super::*;

    pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
        require!(
            ctx.accounts.mmp_mint.data_is_empty(),
            MmpError::MintAlreadyInitialized
        );

        create_and_initialize_mint(
            &ctx.accounts.admin.to_account_info(),
            &ctx.accounts.mmp_mint.to_account_info(),
            &ctx.accounts.mint_authority.key(),
            DECIMALS,
            &ctx.accounts.token_program.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;

        let config = &mut ctx.accounts.protocol_config;
        config.bump = ctx.bumps.protocol_config;
        config.mint_authority_bump = ctx.bumps.mint_authority;
        config.admin = ctx.accounts.admin.key();
        config.mmp_mint = ctx.accounts.mmp_mint.key();
        config.collection_mint = Pubkey::default();
        config.total_supply = TOTAL_SUPPLY_BASE_UNITS;
        config.total_minted = 0;
        config.remaining_supply = TOTAL_SUPPLY_BASE_UNITS;
        config.phase_caps = PHASE_CAPS;
        config.phase_minted = [0; PHASE_COUNT];
        config.phase_paused = [false; PHASE_COUNT];
        config.class_remaining = class_remaining_defaults();
        Ok(())
    }

    pub fn set_phase_pause(ctx: Context<SetPhasePause>, phase: u8, paused: bool) -> Result<()> {
        let phase_index = phase_index(phase)?;
        ctx.accounts.protocol_config.phase_paused[phase_index] = paused;
        Ok(())
    }

    pub fn create_collection(
        ctx: Context<CreateCollection>,
        metadata: NftMetadataArgs,
    ) -> Result<()> {
        let config = &mut ctx.accounts.protocol_config;
        require_keys_eq!(
            ctx.accounts.admin.key(),
            config.admin,
            MmpError::InvalidAdmin
        );
        require!(
            config.collection_mint == Pubkey::default(),
            MmpError::CollectionAlreadyCreated
        );
        require!(
            ctx.accounts.collection_mint.data_is_empty(),
            MmpError::MintAlreadyInitialized
        );
        require_expected_metadata_accounts(
            &ctx.accounts.collection_mint.key(),
            &ctx.accounts.collection_metadata.key(),
            &ctx.accounts.collection_master_edition.key(),
        )?;

        create_and_initialize_mint(
            &ctx.accounts.admin.to_account_info(),
            &ctx.accounts.collection_mint.to_account_info(),
            &ctx.accounts.mint_authority.key(),
            0,
            &ctx.accounts.token_program.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;

        ensure_associated_token_account(
            &ctx.accounts.admin.to_account_info(),
            &ctx.accounts.collection_token_account.to_account_info(),
            &ctx.accounts.admin.to_account_info(),
            &ctx.accounts.collection_mint.to_account_info(),
            &ctx.accounts.associated_token_program.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;

        let mint_authority_bump = [config.mint_authority_bump];
        let mint_authority_signer: &[&[u8]] = &[MINT_AUTHORITY_SEED, &mint_authority_bump];
        let mint_authority_signers: &[&[&[u8]]] = &[mint_authority_signer];
        mint_one_token(
            &ctx.accounts.collection_mint.to_account_info(),
            &ctx.accounts.collection_token_account.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            mint_authority_signers,
        )?;

        let metadata_instruction = metaplex::create_metadata_account_v3(
            ctx.accounts.collection_metadata.key(),
            ctx.accounts.collection_mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.admin.key(),
            ctx.accounts.mint_authority.key(),
            CreateMetadataAccountV3Args {
                data: DataV2 {
                    name: metadata.name,
                    symbol: metadata.symbol,
                    uri: metadata.uri,
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: None,
                    uses: None,
                },
                is_mutable: true,
                collection_details: None,
            },
        );

        invoke_signed(
            &metadata_instruction,
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.collection_metadata.to_account_info(),
                ctx.accounts.collection_mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            mint_authority_signers,
        )?;

        let edition_instruction = metaplex::create_master_edition_v3(
            ctx.accounts.collection_master_edition.key(),
            ctx.accounts.collection_mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.admin.key(),
            ctx.accounts.collection_metadata.key(),
            Some(0),
        );

        invoke_signed(
            &edition_instruction,
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.collection_master_edition.to_account_info(),
                ctx.accounts.collection_mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.collection_metadata.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            mint_authority_signers,
        )?;

        revoke_mint_authority(
            &ctx.accounts.collection_mint.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            mint_authority_signers,
        )?;

        config.collection_mint = ctx.accounts.collection_mint.key();
        Ok(())
    }

    pub fn mint_pass(ctx: Context<MintPass>, phase: u8, metadata: NftMetadataArgs) -> Result<()> {
        let config = &mut ctx.accounts.protocol_config;
        require_keys_eq!(
            ctx.accounts.admin.key(),
            config.admin,
            MmpError::InvalidAdmin
        );
        require!(
            config.collection_mint != Pubkey::default(),
            MmpError::CollectionNotCreated
        );
        require!(
            ctx.accounts.pass_mint.data_is_empty(),
            MmpError::MintAlreadyInitialized
        );
        require_expected_metadata_accounts(
            &ctx.accounts.pass_mint.key(),
            &ctx.accounts.pass_metadata.key(),
            &ctx.accounts.pass_master_edition.key(),
        )?;
        require_keys_eq!(
            ctx.accounts.collection_mint.key(),
            config.collection_mint,
            MmpError::InvalidMint
        );
        require_expected_metadata_accounts(
            &ctx.accounts.collection_mint.key(),
            &ctx.accounts.collection_metadata.key(),
            &ctx.accounts.collection_master_edition.key(),
        )?;

        let phase_index = phase_index(phase)?;
        apply_phase_mint(config, phase_index)?;

        let class_id = assign_class_id(
            &ctx.accounts.slot_hashes.to_account_info(),
            &ctx.accounts.recipient.key(),
            &ctx.accounts.pass_mint.key(),
            config.phase_minted[phase_index],
            &mut config.class_remaining,
        )?;

        create_and_initialize_mint(
            &ctx.accounts.admin.to_account_info(),
            &ctx.accounts.pass_mint.to_account_info(),
            &ctx.accounts.mint_authority.key(),
            0,
            &ctx.accounts.token_program.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;

        ensure_associated_token_account(
            &ctx.accounts.admin.to_account_info(),
            &ctx.accounts.recipient_pass_token.to_account_info(),
            &ctx.accounts.recipient.to_account_info(),
            &ctx.accounts.pass_mint.to_account_info(),
            &ctx.accounts.associated_token_program.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
        )?;

        let mint_authority_bump = [config.mint_authority_bump];
        let mint_authority_signer: &[&[u8]] = &[MINT_AUTHORITY_SEED, &mint_authority_bump];
        let mint_authority_signers: &[&[&[u8]]] = &[mint_authority_signer];
        mint_one_token(
            &ctx.accounts.pass_mint.to_account_info(),
            &ctx.accounts.recipient_pass_token.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            mint_authority_signers,
        )?;

        let metadata_instruction = metaplex::create_metadata_account_v3(
            ctx.accounts.pass_metadata.key(),
            ctx.accounts.pass_mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.admin.key(),
            ctx.accounts.mint_authority.key(),
            CreateMetadataAccountV3Args {
                data: DataV2 {
                    name: metadata.name,
                    symbol: metadata.symbol,
                    uri: metadata.uri,
                    seller_fee_basis_points: 0,
                    creators: None,
                    collection: Some(Collection {
                        verified: false,
                        key: config.collection_mint,
                    }),
                    uses: None,
                },
                is_mutable: true,
                collection_details: None,
            },
        );

        invoke_signed(
            &metadata_instruction,
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.pass_metadata.to_account_info(),
                ctx.accounts.pass_mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            mint_authority_signers,
        )?;

        let edition_instruction = metaplex::create_master_edition_v3(
            ctx.accounts.pass_master_edition.key(),
            ctx.accounts.pass_mint.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.admin.key(),
            ctx.accounts.pass_metadata.key(),
            Some(0),
        );

        invoke_signed(
            &edition_instruction,
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.pass_master_edition.to_account_info(),
                ctx.accounts.pass_mint.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.pass_metadata.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
            ],
            mint_authority_signers,
        )?;

        let verify_instruction = metaplex::set_and_verify_collection(
            ctx.accounts.pass_metadata.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.admin.key(),
            ctx.accounts.mint_authority.key(),
            ctx.accounts.collection_mint.key(),
            ctx.accounts.collection_metadata.key(),
            ctx.accounts.collection_master_edition.key(),
        );

        invoke_signed(
            &verify_instruction,
            &[
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.pass_metadata.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.collection_mint.to_account_info(),
                ctx.accounts.collection_metadata.to_account_info(),
                ctx.accounts.collection_master_edition.to_account_info(),
            ],
            mint_authority_signers,
        )?;

        revoke_mint_authority(
            &ctx.accounts.pass_mint.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
            mint_authority_signers,
        )?;

        let mining_state = &mut ctx.accounts.mining_state;
        mining_state.state_bump = ctx.bumps.mining_state;
        mining_state.custody_bump = ctx.bumps.custody_authority;
        mining_state.pass_mint = ctx.accounts.pass_mint.key();
        mining_state.owner = ctx.accounts.recipient.key();
        mining_state.class_id = class_id;
        mining_state.is_mining = false;
        mining_state.mining_start_ts = 0;
        mining_state.last_claim_ts = 0;
        mining_state.lifetime_claimed = 0;
        Ok(())
    }

    pub fn start_mining(ctx: Context<StartMining>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(now >= 0, MmpError::InvalidTimestamp);
        require!(
            !ctx.accounts.mining_state.is_mining,
            MmpError::AlreadyMining
        );
        require_keys_eq!(
            ctx.accounts.mining_state.pass_mint,
            ctx.accounts.pass_mint.key(),
            MmpError::InvalidMint
        );
        assert_pass_mint(&ctx.accounts.pass_mint)?;
        require!(
            ctx.accounts.owner_pass_token.amount == 1,
            MmpError::InvalidTokenBalance
        );

        let transfer_accounts = Transfer {
            from: ctx.accounts.owner_pass_token.to_account_info(),
            to: ctx.accounts.custody_pass_token.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
            ),
            1,
        )?;

        let mining_state = &mut ctx.accounts.mining_state;
        mining_state.owner = ctx.accounts.owner.key();
        mining_state.is_mining = true;
        mining_state.mining_start_ts = now;
        mining_state.last_claim_ts = now;
        Ok(())
    }

    pub fn claim(ctx: Context<ClaimRewards>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(now >= 0, MmpError::InvalidTimestamp);
        require!(ctx.accounts.mining_state.is_mining, MmpError::NotMining);
        require_keys_eq!(
            ctx.accounts.mining_state.owner,
            ctx.accounts.owner.key(),
            MmpError::InvalidOwner
        );
        assert_pass_mint(&ctx.accounts.pass_mint)?;
        settle_rewards(
            &mut ctx.accounts.protocol_config,
            &mut ctx.accounts.mining_state,
            now,
            &ctx.accounts.owner_mmp_token.to_account_info(),
            &ctx.accounts.mmp_mint.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
        )?;
        Ok(())
    }

    pub fn stop_mining(ctx: Context<StopMining>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(now >= 0, MmpError::InvalidTimestamp);
        require!(ctx.accounts.mining_state.is_mining, MmpError::NotMining);
        require_keys_eq!(
            ctx.accounts.mining_state.owner,
            ctx.accounts.owner.key(),
            MmpError::InvalidOwner
        );
        assert_pass_mint(&ctx.accounts.pass_mint)?;
        require!(
            ctx.accounts.custody_pass_token.amount == 1,
            MmpError::InvalidCustody
        );

        settle_rewards(
            &mut ctx.accounts.protocol_config,
            &mut ctx.accounts.mining_state,
            now,
            &ctx.accounts.owner_mmp_token.to_account_info(),
            &ctx.accounts.mmp_mint.to_account_info(),
            &ctx.accounts.mint_authority.to_account_info(),
            &ctx.accounts.token_program.to_account_info(),
        )?;

        let custody_bump = [ctx.accounts.mining_state.custody_bump];
        let custody_signer: &[&[u8]] = &[
            CUSTODY_SEED,
            ctx.accounts.pass_mint.key().as_ref(),
            &custody_bump,
        ];
        let signer: &[&[&[u8]]] = &[custody_signer];
        let transfer_accounts = Transfer {
            from: ctx.accounts.custody_pass_token.to_account_info(),
            to: ctx.accounts.owner_pass_token.to_account_info(),
            authority: ctx.accounts.custody_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_accounts,
                signer,
            ),
            1,
        )?;

        ctx.accounts.mining_state.is_mining = false;
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct NftMetadataArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + ProtocolConfig::INIT_SPACE,
        seeds = [PROTOCOL_CONFIG_SEED],
        bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Created and initialized as the immutable MMP mint in this instruction.
    #[account(mut, signer)]
    pub mmp_mint: UncheckedAccount<'info>,
    /// CHECK: PDA signer only.
    #[account(seeds = [MINT_AUTHORITY_SEED], bump)]
    pub mint_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetPhasePause<'info> {
    #[account(mut, seeds = [PROTOCOL_CONFIG_SEED], bump = protocol_config.bump, has_one = admin)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut, seeds = [PROTOCOL_CONFIG_SEED], bump = protocol_config.bump, has_one = admin)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: PDA signer only.
    #[account(seeds = [MINT_AUTHORITY_SEED], bump = protocol_config.mint_authority_bump)]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK: Created as the collection mint in this instruction.
    #[account(mut, signer)]
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: Metaplex metadata PDA validated in handler.
    #[account(mut)]
    pub collection_metadata: UncheckedAccount<'info>,
    /// CHECK: Metaplex edition PDA validated in handler.
    #[account(mut)]
    pub collection_master_edition: UncheckedAccount<'info>,
    /// CHECK: ATA created on demand for the collection NFT.
    #[account(mut)]
    pub collection_token_account: UncheckedAccount<'info>,
    /// CHECK: Verified against the Metaplex program ID constant.
    #[account(address = metaplex::TOKEN_METADATA_PROGRAM_ID)]
    pub token_metadata_program: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct MintPass<'info> {
    #[account(mut, seeds = [PROTOCOL_CONFIG_SEED], bump = protocol_config.bump, has_one = admin)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: PDA signer only.
    #[account(seeds = [MINT_AUTHORITY_SEED], bump = protocol_config.mint_authority_bump)]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK: Created as the pass mint in this instruction.
    #[account(mut, signer)]
    pub pass_mint: UncheckedAccount<'info>,
    /// CHECK: PDA metadata account validated in handler.
    #[account(mut)]
    pub pass_metadata: UncheckedAccount<'info>,
    /// CHECK: PDA master edition account validated in handler.
    #[account(mut)]
    pub pass_master_edition: UncheckedAccount<'info>,
    #[account(mut)]
    pub recipient: SystemAccount<'info>,
    /// CHECK: ATA created on demand for the recipient.
    #[account(mut)]
    pub recipient_pass_token: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + MiningState::INIT_SPACE,
        seeds = [MINING_STATE_SEED, pass_mint.key().as_ref()],
        bump
    )]
    pub mining_state: Account<'info, MiningState>,
    /// CHECK: PDA signer used later as NFT custody authority.
    #[account(seeds = [CUSTODY_SEED, pass_mint.key().as_ref()], bump)]
    pub custody_authority: UncheckedAccount<'info>,
    /// CHECK: slot hashes sysvar is deserialized manually for weighted randomness.
    #[account(address = anchor_lang::solana_program::sysvar::slot_hashes::id())]
    pub slot_hashes: UncheckedAccount<'info>,
    #[account(address = protocol_config.collection_mint)]
    pub collection_mint: Account<'info, Mint>,
    /// CHECK: Collection metadata PDA validated in handler.
    pub collection_metadata: UncheckedAccount<'info>,
    /// CHECK: Collection master edition PDA validated in handler.
    pub collection_master_edition: UncheckedAccount<'info>,
    /// CHECK: Verified against the Metaplex program ID constant.
    #[account(address = metaplex::TOKEN_METADATA_PROGRAM_ID)]
    pub token_metadata_program: UncheckedAccount<'info>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StartMining<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub pass_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [MINING_STATE_SEED, pass_mint.key().as_ref()],
        bump = mining_state.state_bump,
        has_one = pass_mint
    )]
    pub mining_state: Account<'info, MiningState>,
    #[account(mut, token::mint = pass_mint, token::authority = owner)]
    pub owner_pass_token: Account<'info, TokenAccount>,
    /// CHECK: PDA signer that owns the custody ATA.
    #[account(seeds = [CUSTODY_SEED, pass_mint.key().as_ref()], bump = mining_state.custody_bump)]
    pub custody_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = pass_mint,
        associated_token::authority = custody_authority
    )]
    pub custody_pass_token: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut, seeds = [PROTOCOL_CONFIG_SEED], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(address = protocol_config.mmp_mint)]
    pub mmp_mint: Account<'info, Mint>,
    #[account(mut)]
    pub pass_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [MINING_STATE_SEED, pass_mint.key().as_ref()],
        bump = mining_state.state_bump,
        has_one = pass_mint
    )]
    pub mining_state: Account<'info, MiningState>,
    /// CHECK: PDA signer only.
    #[account(seeds = [MINT_AUTHORITY_SEED], bump = protocol_config.mint_authority_bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = mmp_mint,
        associated_token::authority = owner
    )]
    pub owner_mmp_token: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StopMining<'info> {
    #[account(mut, seeds = [PROTOCOL_CONFIG_SEED], bump = protocol_config.bump)]
    pub protocol_config: Account<'info, ProtocolConfig>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(address = protocol_config.mmp_mint)]
    pub mmp_mint: Account<'info, Mint>,
    #[account(mut)]
    pub pass_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [MINING_STATE_SEED, pass_mint.key().as_ref()],
        bump = mining_state.state_bump,
        has_one = pass_mint
    )]
    pub mining_state: Account<'info, MiningState>,
    /// CHECK: PDA signer only.
    #[account(seeds = [MINT_AUTHORITY_SEED], bump = protocol_config.mint_authority_bump)]
    pub mint_authority: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = mmp_mint,
        associated_token::authority = owner
    )]
    pub owner_mmp_token: Account<'info, TokenAccount>,
    /// CHECK: PDA signer that owns the custody ATA.
    #[account(seeds = [CUSTODY_SEED, pass_mint.key().as_ref()], bump = mining_state.custody_bump)]
    pub custody_authority: UncheckedAccount<'info>,
    #[account(mut, associated_token::mint = pass_mint, associated_token::authority = custody_authority)]
    pub custody_pass_token: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = pass_mint,
        associated_token::authority = owner
    )]
    pub owner_pass_token: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

fn class_remaining_defaults() -> [u32; CLASS_COUNT] {
    let mut values = [0u32; CLASS_COUNT];
    let mut index = 0;
    while index < CLASS_COUNT {
        values[index] = CLASS_TABLE[index].count;
        index += 1;
    }
    values
}

fn phase_index(phase: u8) -> Result<usize> {
    let idx = usize::from(phase);
    require!(idx < PHASE_COUNT, MmpError::InvalidPhase);
    Ok(idx)
}

fn apply_phase_mint(config: &mut ProtocolConfig, phase_index: usize) -> Result<()> {
    require!(!config.phase_paused[phase_index], MmpError::PhasePaused);
    require!(
        config.phase_minted[phase_index] < config.phase_caps[phase_index],
        MmpError::PhaseCapExceeded
    );
    config.phase_minted[phase_index] = config.phase_minted[phase_index]
        .checked_add(1)
        .ok_or_else(|| error!(MmpError::MathOverflow))?;
    Ok(())
}

fn assign_class_id(
    slot_hashes_info: &AccountInfo,
    recipient: &Pubkey,
    pass_mint: &Pubkey,
    phase_mint_count: u32,
    class_remaining: &mut [u32; CLASS_COUNT],
) -> Result<u8> {
    let slot_hashes = SlotHashes::from_account_info(slot_hashes_info)
        .map_err(|_| error!(MmpError::InvalidSlotHashes))?;
    let (_, recent_hash) = slot_hashes
        .iter()
        .next()
        .ok_or_else(|| error!(MmpError::InvalidSlotHashes))?;
    let entropy = hashv(&[
        recent_hash.as_ref(),
        recipient.as_ref(),
        pass_mint.as_ref(),
        &phase_mint_count.to_le_bytes(),
    ]);
    let mut ticket_bytes = [0u8; 8];
    ticket_bytes.copy_from_slice(&entropy.to_bytes()[..8]);
    let ticket = u64::from_le_bytes(ticket_bytes);
    draw_class_id(ticket, class_remaining)
}

fn draw_class_id(ticket: u64, class_remaining: &mut [u32; CLASS_COUNT]) -> Result<u8> {
    let total_remaining: u64 = class_remaining.iter().map(|count| u64::from(*count)).sum();
    require!(total_remaining > 0, MmpError::CapacityExhausted);

    let mut cursor = ticket % total_remaining;
    for (index, remaining) in class_remaining.iter_mut().enumerate() {
        let remaining_u64 = u64::from(*remaining);
        if remaining_u64 == 0 {
            continue;
        }
        if cursor < remaining_u64 {
            *remaining = remaining
                .checked_sub(1)
                .ok_or_else(|| error!(MmpError::MathOverflow))?;
            return Ok(CLASS_TABLE[index].class_id);
        }
        cursor = cursor
            .checked_sub(remaining_u64)
            .ok_or_else(|| error!(MmpError::MathOverflow))?;
    }

    err!(MmpError::CapacityExhausted)
}

fn settle_rewards<'info>(
    config: &mut Account<'info, ProtocolConfig>,
    mining_state: &mut Account<'info, MiningState>,
    now_ts: i64,
    owner_mmp_token: &AccountInfo<'info>,
    mmp_mint: &AccountInfo<'info>,
    mint_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> Result<u64> {
    require!(
        now_ts >= mining_state.last_claim_ts,
        MmpError::InvalidTimestamp
    );
    let elapsed = (now_ts - mining_state.last_claim_ts) as u64;
    let class =
        class_config(mining_state.class_id).ok_or_else(|| error!(MmpError::InvalidClass))?;
    let theoretical_reward =
        calculate_reward(elapsed, class.multiplier).map_err(map_reward_math_error)?;
    let minted_reward = cap_reward_to_remaining_supply(theoretical_reward, config.remaining_supply);

    if minted_reward > 0 {
        let mint_authority_bump = [config.mint_authority_bump];
        let mint_authority_signer: &[&[u8]] = &[MINT_AUTHORITY_SEED, &mint_authority_bump];
        let signer: &[&[&[u8]]] = &[mint_authority_signer];
        token::mint_to(
            CpiContext::new_with_signer(
                token_program.clone(),
                MintTo {
                    mint: mmp_mint.clone(),
                    to: owner_mmp_token.clone(),
                    authority: mint_authority.clone(),
                },
                signer,
            ),
            minted_reward,
        )?;

        config.total_minted = config
            .total_minted
            .checked_add(minted_reward)
            .ok_or_else(|| error!(MmpError::MathOverflow))?;
        config.remaining_supply = config
            .remaining_supply
            .checked_sub(minted_reward)
            .ok_or_else(|| error!(MmpError::MathOverflow))?;
        mining_state.lifetime_claimed = mining_state
            .lifetime_claimed
            .checked_add(minted_reward)
            .ok_or_else(|| error!(MmpError::MathOverflow))?;
    }

    mining_state.last_claim_ts = now_ts;
    Ok(minted_reward)
}

fn create_and_initialize_mint<'info>(
    payer: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    mint_authority: &Pubkey,
    decimals: u8,
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<()> {
    let rent = Rent::get()?;
    let mint_len = SplMint::LEN;
    let lamports = rent.minimum_balance(mint_len);
    invoke(
        &system_instruction::create_account(
            payer.key,
            mint.key,
            lamports,
            mint_len as u64,
            token_program.key,
        ),
        &[payer.clone(), mint.clone(), system_program.clone()],
    )?;
    invoke(
        &spl_token::instruction::initialize_mint2(
            token_program.key,
            mint.key,
            mint_authority,
            None,
            decimals,
        )?,
        &[mint.clone(), token_program.clone()],
    )?;
    Ok(())
}

fn ensure_associated_token_account<'info>(
    payer: &AccountInfo<'info>,
    associated_token: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    associated_token_program: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
) -> Result<()> {
    if associated_token.data_is_empty() {
        associated_token::create(CpiContext::new(
            associated_token_program.clone(),
            associated_token::Create {
                payer: payer.clone(),
                associated_token: associated_token.clone(),
                authority: authority.clone(),
                mint: mint.clone(),
                system_program: system_program.clone(),
                token_program: token_program.clone(),
            },
        ))?;
    }
    Ok(())
}

fn mint_one_token<'info>(
    mint: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    mint_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    token::mint_to(
        CpiContext::new_with_signer(
            token_program.clone(),
            MintTo {
                mint: mint.clone(),
                to: destination.clone(),
                authority: mint_authority.clone(),
            },
            signer_seeds,
        ),
        1,
    )?;
    Ok(())
}

fn revoke_mint_authority<'info>(
    mint: &AccountInfo<'info>,
    mint_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    token::set_authority(
        CpiContext::new_with_signer(
            token_program.clone(),
            SetAuthority {
                current_authority: mint_authority.clone(),
                account_or_mint: mint.clone(),
            },
            signer_seeds,
        ),
        AuthorityType::MintTokens,
        None,
    )?;
    Ok(())
}

fn require_expected_metadata_accounts(
    mint: &Pubkey,
    metadata: &Pubkey,
    edition: &Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *metadata,
        metaplex::metadata_pda(mint),
        MmpError::InvalidMetadataPda
    );
    require_keys_eq!(
        *edition,
        metaplex::master_edition_pda(mint),
        MmpError::InvalidEditionPda
    );
    Ok(())
}

fn assert_pass_mint(pass_mint: &Account<Mint>) -> Result<()> {
    require!(pass_mint.decimals == 0, MmpError::InvalidPassMint);
    require!(pass_mint.supply == 1, MmpError::InvalidPassMint);
    Ok(())
}

fn map_reward_math_error(error: RewardMathError) -> Error {
    match error {
        RewardMathError::Overflow | RewardMathError::DivisionByZero => {
            error!(MmpError::MathOverflow)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_caps_are_enforced() {
        let mut config = ProtocolConfig {
            bump: 0,
            mint_authority_bump: 0,
            admin: Pubkey::default(),
            mmp_mint: Pubkey::default(),
            collection_mint: Pubkey::default(),
            total_supply: TOTAL_SUPPLY_BASE_UNITS,
            total_minted: 0,
            remaining_supply: TOTAL_SUPPLY_BASE_UNITS,
            phase_caps: PHASE_CAPS,
            phase_minted: [PHASE_CAPS[0], 0, 0],
            phase_paused: [false; PHASE_COUNT],
            class_remaining: class_remaining_defaults(),
        };
        assert!(apply_phase_mint(&mut config, 0).is_err());
    }

    #[test]
    fn class_draw_skips_exhausted_buckets() {
        let mut remaining = [0, 0, 1, 0, 0, 0, 0];
        let class_id = draw_class_id(0, &mut remaining).unwrap();
        assert_eq!(class_id, 2);
        assert_eq!(remaining[2], 0);
    }

    #[test]
    fn class_draw_respects_weighted_capacity() {
        let mut remaining = [2, 1, 0, 0, 0, 0, 0];
        assert_eq!(draw_class_id(0, &mut remaining).unwrap(), 0);
        assert_eq!(draw_class_id(1, &mut remaining).unwrap(), 0);
        assert_eq!(draw_class_id(2, &mut remaining).unwrap(), 1);
    }
}
