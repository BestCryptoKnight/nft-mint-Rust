use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_lang::solana_program::{
    program::{
        invoke_signed, invoke,
    },
    system_instruction::{
        transfer,
        create_account,
    },
    sysvar::{
        rent::Rent
    },

};
use anchor_spl::{
    associated_token::{self, AssociatedToken, Create},
    token::{self, Mint, Token, MintTo},
};
use mpl_token_metadata::{
    instruction::{
        create_metadata_accounts_v2, create_master_edition_v3,
    },
    state::{
        Creator,
    },

};


declare_id!("HW6rBHPcBYePNBgkEFRvRy8YhS7S7NnJ2rEeSoj7Q6jG");

#[program]
pub mod nft_maker {
    use super::*;

    ///initialize program
    /// config_nonce: The PDA nonce of config account
    /// vault_nonce: The PDA nonce of payer vault account
    /// authority: The pubkey of minting permission
    /// amount: the count of lamports that transfer from initializer to payer vault
    pub fn initialize(
        ctx: Context<Initialize>,
        config_nonce: u8,
        vault_nonce: u8,
        authority: Pubkey,
        amount: u64,
    ) -> ProgramResult {
        //initialize configuration
        let config = &mut ctx.accounts.nft_mint_settings;
        config.nft_count = 0;
        config.payer_vault = *ctx.accounts.payer_vault.key;
        config.authority = authority;
        config.config_nonce = config_nonce;
        config.vault_nonce = vault_nonce;

        // transfer sol token to payer_vault if amount != 0
        if amount != 0 {
            invoke(
                &transfer(
                    ctx.accounts.signer.to_account_info().key,
                    ctx.accounts.payer_vault.key,
                    amount,
                ),
                &[
                    ctx.accounts.signer.to_account_info(),
                    ctx.accounts.payer_vault.clone(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }

        Ok(())
    }

    ///minting NFT for Users
    /// name: the name of NFT
    /// symbol: the symbol of NFT
    /// uri: URI to the external JSON representing the asset
    /// seller_fee_basis_points: royalties percentage awarded to creators
    /// immutable: Whether or not the data struct is mutable, default is not
    pub fn minting_nft(
        ctx: Context<MintingNFT>,
        name: String,
        symbol: String,
        uri: String,
        seller_fee_basis_points: u16,
        immutable: bool,
        mint_nonce: u8,
    ) -> ProgramResult {

        let metaplex_program_id = mpl_token_metadata::ID;

        let config = &ctx.accounts.nft_mint_settings;
        let vault_seeds = &[
            config.to_account_info().key.as_ref(),
            &[config.config_nonce],
        ];

        let nft_id = symbol.clone();
        let mint_seeds = &[
            nft_id.as_bytes(),
            &[mint_nonce],
        ];

        let pda_signer = &[&vault_seeds[..], &mint_seeds[..]];

        //create mint account, the payer must be payer_vault
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(Mint::LEN);
        invoke_signed(
            &create_account(
                ctx.accounts.payer_vault.key,
                ctx.accounts.mint.key,
                lamports,
                Mint::LEN as u64,
                ctx.accounts.token_program.to_account_info().key,
            ),
            &[
                ctx.accounts.payer_vault.clone(),
                ctx.accounts.mint.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
            pda_signer,
        )?;

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let accounts = anchor_spl::token::InitializeMint {
            mint: ctx.accounts.mint.clone(),
            rent: ctx.accounts.rent.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, accounts).with_signer(pda_signer);
        token::initialize_mint(
            cpi_ctx,
            0,
            ctx.accounts.payer_vault.key,
            Option::<&Pubkey>::Some(ctx.accounts.payer_vault.key),
        )?;

        //create associated token account for user
        if ctx.accounts.recipient_token.data_is_empty() {
            let cpi_accounts = Create {
                payer: ctx.accounts.payer_vault.clone(),
                associated_token: ctx.accounts.recipient_token.clone(),
                authority: ctx.accounts.recipient.clone(),
                rent: ctx.accounts.rent.to_account_info(),
                mint: ctx.accounts.mint.clone(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            };
            let cpi_program = ctx.accounts.associated_token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts).with_signer(pda_signer);
            associated_token::create(cpi_ctx)?;
        }

        //minting NFT for user
        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.clone(),
            to: ctx.accounts.recipient_token.clone(),
            authority: ctx.accounts.payer_vault.clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts).with_signer(pda_signer);
        token::mint_to(cpi_ctx, 1)?;

        //create derive metadata account
        let metadata_seeds = &[
            "metadata".as_bytes(),
            metaplex_program_id.as_ref(),
            ctx.accounts.mint.key.as_ref(),
        ];
        let (metadata_account, _pda) =
            Pubkey::find_program_address(metadata_seeds, &metaplex_program_id);

        let creators = vec![
            Creator {
                address: *ctx.accounts.payer_vault.key,
                verified: false,
                share: 0,
            },
            Creator {
                address: *ctx.accounts.recipient.key,
                verified: false,
                share: 100,
            }
        ];

        let create_metadata_account_ix = create_metadata_accounts_v2(
            metaplex_program_id,
            metadata_account,
            *ctx.accounts.mint.key,
            *ctx.accounts.payer_vault.key,
            *ctx.accounts.payer_vault.key,
            *ctx.accounts.payer_vault.key,
            name,
            symbol,
            uri,
            Some(creators),
            seller_fee_basis_points,
            true,
            !immutable,
            None,
            None
        );

        invoke_signed(
            &create_metadata_account_ix,
            &[
                ctx.accounts.metadata.clone(),
                ctx.accounts.mint.clone(),
                ctx.accounts.payer_vault.clone(),
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),

            ],
            pda_signer,
        )?;

        //Create derive Master Edition account
        let master_edition_seeds = &[
            "metadata".as_bytes(),
            metaplex_program_id.as_ref(),
            ctx.accounts.mint.key.as_ref(),
            "edition".as_bytes(),
        ];
        let (master_edition_account, _pda) =
            Pubkey::find_program_address(master_edition_seeds, &metaplex_program_id);

        let create_master_edition_account_ix = create_master_edition_v3(
            metaplex_program_id,
            master_edition_account,
            *ctx.accounts.mint.key,
            *ctx.accounts.payer_vault.key,
            *ctx.accounts.payer_vault.key,
            metadata_account,
            *ctx.accounts.payer_vault.key,
            Some(0),
        );

        invoke_signed(
            &create_master_edition_account_ix,
            &[
                ctx.accounts.masteredition.clone(),
                ctx.accounts.metadata.clone(),
                ctx.accounts.mint.clone(),
                ctx.accounts.payer_vault.clone(),
                ctx.accounts.token_metadata_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),

            ],
            pda_signer,
        )?;

        ctx.accounts.nft_mint_settings.nft_count += 1;

        //emit mint event to client
        emit!(MintEvent {
            mint: ctx.accounts.mint.key.to_string(),
            recipient: ctx.accounts.recipient.key.to_string(),
            nft_count: ctx.accounts.nft_mint_settings.nft_count.to_string(),
            status: "ok".to_string(),
        });

        Ok(())
    }
}

///The accounts of Initialize instruction
/// signer: the initializer, the signer and fee payer
/// payer_vault: the vault account (PDA) for payer
/// nft_mint_settings: the account (PDA) for saving configuration
/// system_program: System program
/// rent: rent info
#[derive(Accounts)]
#[instruction(config_nonce: u8, vault_nonce: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(
        mut,
        seeds = [nft_mint_settings.to_account_info().key.as_ref()],
        bump = vault_nonce,
    )]
    pub payer_vault: AccountInfo<'info>,

    #[account(
        init, payer = signer,
        seeds = [b"nft-maker".as_ref()],
        bump = config_nonce,
    )]
    pub nft_mint_settings: Box<Account<'info, NFTMintSettings>>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

    #[account(address = solana_program::sysvar::rent::ID)]
    pub rent: Sysvar<'info, Rent>,
}


#[account]
#[derive(Default)]
pub struct NFTMintSettings {
    //The PDA nonce of config account
    pub config_nonce: u8,
    //The PDA nonce of payer vault account
    pub vault_nonce: u8,
    //The account of minting permission
    pub authority: Pubkey,
    //The PDA of payment account
    pub payer_vault: Pubkey,
    //Number of NFT's that had been minted
    pub nft_count: u64,
}

///The accounts of MintingNFT instruction
/// signer: the account of caller that must have permission to invoke this instruction.
/// recipient: the account for receiving NFT
/// recipient_token: the associated token accounts for NFT mint
/// payer_vault: the PDA of payment account
/// nft_mint_settings: the PDA of save NFT settings
/// mint: the NFT mint account
/// metadata: the account for NFT metadata
/// masteredition: the account for NFT master edition metadata
/// token_metadata_program: the metaplex metadata program
/// token_program: the token program of SPL
/// associated_token_program: the associated token program of SPL
/// system_program: system program
/// clock: the clock information
/// rent: the rent information
#[derive(Accounts)]
pub struct MintingNFT<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub recipient: AccountInfo<'info>,

    #[account(mut)]
    pub recipient_token: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [nft_mint_settings.to_account_info().key.as_ref()],
        bump = nft_mint_settings.vault_nonce,
    )]
    pub payer_vault: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"nft-maker".as_ref()],
        bump = nft_mint_settings.config_nonce,
        constraint = nft_mint_settings.payer_vault == payer_vault.key() @ErrorCode::PayerVaultMismatch,
        constraint = nft_mint_settings.authority == signer.key() @ErrorCode::Unauthorized,
    )]
    pub nft_mint_settings: Box<Account<'info, NFTMintSettings>>,

    #[account(mut)]
    pub mint: AccountInfo<'info>,

    #[account(mut)]
    pub metadata: AccountInfo<'info>,

    #[account(mut)]
    pub masteredition: AccountInfo<'info>,

    #[account(address = mpl_token_metadata::ID)]
    pub token_metadata_program: AccountInfo<'info>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,

    #[account(address = associated_token::ID)]
    pub associated_token_program: Program<'info, AssociatedToken>,

    #[account(address = system_program::ID)]
    pub system_program: Program<'info, System>,

    #[account(address = solana_program::sysvar::clock::ID)]
    pub clock: Sysvar<'info, Clock>,

    #[account(address = solana_program::sysvar::rent::ID)]
    pub rent: Sysvar<'info, Rent>,

}

#[event]
pub struct MintEvent {
    #[index]
    pub mint: String,
    #[index]
    pub recipient: String,
    #[index]
    pub status: String,
    pub nft_count: String,
}

#[error]
pub enum ErrorCode {
    #[msg("Payer vault account mismatch.")]
    PayerVaultMismatch,
    #[msg("Invalid owner.")]
    InvalidOwner,
    #[msg("You do not have sufficient permissions to perform this action.")]
    Unauthorized,
    #[msg("Invalid associated token address. Did you provide the correct address?")]
    InvalidAssociatedTokenAddress,

}