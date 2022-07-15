use anchor_lang::prelude::*;
use anchor_lang::system_program::{Transfer, transfer};
use anchor_spl::{associated_token::AssociatedToken, token::{Mint, Token, TokenAccount, transfer as token_transfer, Transfer as Token_Transfer}};

declare_id!("3ERnPj1gQnKzagvWCjqZGskJszcLhunXuLfYqjuSvxBW");

const ESCROW_PDA_SEED: &[u8] = b"escrow_seed";

//const VAULT_PDA_SEED: &[u8] = b"vault_seed";

const FIND_PROGRAM_SEED: &[u8] = b"FIND_PROGRAM";

const TOKEN_ESCROW_PDA_SEED: &[u8] = b"token_escrow_seed";


#[program]
pub mod token_withdraw {
    use super::*;

    pub fn initialize_native_sol(ctx: Context<InitializeNative>, start_time: u64, amount: u64) -> Result<()> {
        msg!("into initiallize native sol");
        ctx.accounts.escrow_account.sender_account = *ctx.accounts.sender_account.key;
        ctx.accounts.escrow_account.receiver_account = *ctx.accounts.receiver_account.key;
        ctx.accounts.escrow_account.start_time = start_time;
        ctx.accounts.escrow_account.amount = amount;

        if !ctx.accounts.sender_account.is_signer {
            return Err(error!(ErrorCode::AccountNotSigner));
        }

        let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(&[FIND_PROGRAM_SEED], ctx.program_id);
        let pda_signer_seed:&[&[&[_]]] = &[&[&FIND_PROGRAM_SEED, &[vault_authority_bump]]];

        // Perform the actual transfer
        let ix = Transfer{
            from: ctx.accounts.sender_account.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            ix,
            pda_signer_seed,
        );
        
        transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn withdraw_native_sol(ctx: Context<WithdrawNative>, amount: u64) -> Result<()> {

        if ctx.accounts.escrow_account.receiver_account.key() != *ctx.accounts.receiver_account.key {
            return Err(error!(ErrorCode::InvalidProgramExecutable));
        }

        if ctx.accounts.escrow_account.start_time + 2 > Clock::get()?.unix_timestamp as u64{ // 24 hours not passed yet (24*60*60)
            return Err(error!(ErrorCode::InvalidProgramExecutable));
        }

        let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(&[FIND_PROGRAM_SEED], ctx.program_id);
        let pda_signer_seed:&[&[&[_]]] = &[&[&FIND_PROGRAM_SEED, &[vault_authority_bump]]];

        // let inner = vec![b"vault_transfer".as_ref(), ctx.accounts.sender_account.key.as_ref()];
        // let pda_signer_seed = vec![inner.as_slice()];
        
        let ix = Transfer{
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.receiver_account.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            ix,
            pda_signer_seed,
        );
        
        transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn intialize_fungible_token(ctx: Context<InitializeFungibleToken>, start_time: u64, amount: u64) -> Result<()> {
        ctx.accounts.escrow_account.sender_account = *ctx.accounts.sender_account.key;
        ctx.accounts.escrow_account.receiver_account = *ctx.accounts.receiver_account.key;
        ctx.accounts.escrow_account.token_mint = ctx.accounts.mint.key();
        ctx.accounts.escrow_account.start_time = start_time;
        ctx.accounts.escrow_account.amount_token = amount;
        

        let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(&[FIND_PROGRAM_SEED], ctx.program_id);
        let pda_signer_seed:&[&[&[_]]] = &[&[&FIND_PROGRAM_SEED, &[vault_authority_bump]]];

        let transfer_instruction = Token_Transfer{
            from: ctx.accounts.sender_associated_info.to_account_info(),
            to: ctx.accounts.vault_associated_info.to_account_info(),
            authority: ctx.accounts.sender_account.to_account_info()
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            transfer_instruction,
            pda_signer_seed
        );
        
        token_transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn withdraw_fungible_token(ctx: Context<WithdrawFungibleToken>, amount: u64) -> Result<()> {
        if ctx.accounts.escrow_account.receiver_account.key() != *ctx.accounts.receiver_account.key {
            return Err(error!(ErrorCode::InvalidProgramExecutable));
        }

        if ctx.accounts.escrow_account.start_time + 2 > Clock::get()?.unix_timestamp as u64{ // 24 hours not passed yet (24*60*60)
            return Err(error!(ErrorCode::InvalidProgramExecutable));
        }

        let (_vault_authority, vault_authority_bump) = Pubkey::find_program_address(&[FIND_PROGRAM_SEED], ctx.program_id);
        let pda_signer_seed:&[&[&[_]]] = &[&[&FIND_PROGRAM_SEED, &[vault_authority_bump]]];
        
        let transfer_ix = Token_Transfer{
            from: ctx.accounts.vault_associated_info.to_account_info(),
            to: ctx.accounts.receiver_associated_info.to_account_info(),
            authority: ctx.accounts.vault.to_account_info()
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            transfer_ix,
            pda_signer_seed,
        );
        
        token_transfer(cpi_ctx, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeNative<'info> {
    #[account(
        init, 
        payer = sender_account, 
        space = 8 + 32 + 32 + 8 + 8, 
        seeds = [ESCROW_PDA_SEED, sender_account.key().as_ref()], 
        bump 
        )
    ]
    pub escrow_account: Account<'info, EscrowNative>,
    #[account(mut)]
    pub sender_account: Signer<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub receiver_account: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct WithdrawNative<'info> {
    #[account(
        seeds = [ESCROW_PDA_SEED, sender_account.key().as_ref()], 
        bump,
    )]
    pub escrow_account: Account<'info, EscrowNative>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub sender_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub receiver_account: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeFungibleToken<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        init_if_needed,
        payer = sender_account,
        space = 8 + 32 + 32 + 32 + 8 + 8,
        seeds = [TOKEN_ESCROW_PDA_SEED, sender_account.key().as_ref()],
        bump
        )
    ]
    pub escrow_account: Account<'info, EscrowFungibleToken>,
    #[account(mut)]
    pub sender_associated_info: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = sender_account,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_associated_info: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub sender_account: Signer<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub vault: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub receiver_account: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info,Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct WithdrawFungibleToken<'info> {
    #[account(
        seeds = [TOKEN_ESCROW_PDA_SEED, sender_account.key().as_ref()],
        bump 
        )
    ]
    pub escrow_account: Account<'info, EscrowFungibleToken>,
    #[account(
        init_if_needed,
        payer = receiver_account,
        associated_token::mint = mint,
        associated_token::authority = receiver_account,
        )
    ]
    pub receiver_associated_info: Account<'info, TokenAccount>,
    #[account(
        associated_token::mint = mint,
        associated_token::authority = vault,
        mut
    )]
    pub vault_associated_info: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub sender_account: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub receiver_account: Signer<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(mut)]
    pub vault: Signer<'info>,
    pub mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info,Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct EscrowNative {
    pub sender_account: Pubkey,
    pub receiver_account: Pubkey,
    pub start_time: u64,
    pub amount: u64
}

#[account]
pub struct EscrowFungibleToken {
    pub sender_account: Pubkey,
    pub receiver_account: Pubkey,
    pub token_mint: Pubkey,
    pub start_time: u64,
    pub amount_token: u64,
}
