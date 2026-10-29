use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, Token, TokenAccount, Transfer};

declare_id!("HC2YZNhkvvHPP2Bt4KwXe3hayNQW2JBM3zpbPjSfwG4q");
const MAX_DEADLINE_SECONDS: i64 = 31_536_000;

#[program]
pub mod agentpay_escrow {
    use super::*;
    pub fn initialize_escrow(
        ctx: Context<InitializeEscrow>,
        order_id: [u8; 32],
        amount: u64,
        deadline: i64,
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(amount > 0, EscrowError::InvalidAmount);
        require!(deadline > now, EscrowError::DeadlineInPast);
        require!(
            deadline
                <= now
                    .checked_add(MAX_DEADLINE_SECONDS)
                    .ok_or(EscrowError::MathOverflow)?,
            EscrowError::DeadlineTooFar
        );
        let escrow = &mut ctx.accounts.escrow;
        escrow.buyer = ctx.accounts.buyer.key();
        escrow.seller = ctx.accounts.seller.key();
        escrow.mint = ctx.accounts.mint.key();
        escrow.amount = amount;
        escrow.deadline = deadline;
        escrow.order_id = order_id;
        escrow.status = EscrowStatus::Created;
        escrow.bump = ctx.bumps.escrow;
        escrow.vault_bump = ctx.bumps.vault_authority;
        escrow.release_arbiter = ctx.accounts.buyer.key();
        escrow.bump_version = 1;
        emit!(EscrowCreated {
            order_id,
            escrow: escrow.key(),
            buyer: escrow.buyer,
            seller: escrow.seller,
            amount,
            deadline,
            mint: escrow.mint
        });
        Ok(())
    }
    pub fn fund_escrow(ctx: Context<FundEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Created,
            EscrowError::InvalidStatus
        );
        require_keys_eq!(
            ctx.accounts.buyer.key(),
            escrow.buyer,
            EscrowError::Unauthorized
        );
        require_keys_eq!(ctx.accounts.mint.key(), escrow.mint, EscrowError::WrongMint);
        require_keys_eq!(ctx.accounts.vault.mint, escrow.mint, EscrowError::WrongMint);
        let cpi_accounts = Transfer {
            from: ctx.accounts.buyer_token_account.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        };
        token::transfer(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
            escrow.amount,
        )?;
        escrow.status = EscrowStatus::Funded;
        emit!(EscrowFunded {
            order_id: escrow.order_id,
            escrow: escrow.key(),
            amount: escrow.amount,
            vault: ctx.accounts.vault.key()
        });
        Ok(())
    }
    pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Funded,
            EscrowError::InvalidStatus
        );
        require_keys_eq!(
            ctx.accounts.release_authority.key(),
            escrow.release_arbiter,
            EscrowError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.seller_token_account.owner,
            escrow.seller,
            EscrowError::InvalidTokenAccount
        );
        require_keys_eq!(
            ctx.accounts.seller_token_account.mint,
            escrow.mint,
            EscrowError::WrongMint
        );
        let escrow_key = escrow.key();
        let seeds = &[
            b"vault-authority",
            escrow_key.as_ref(),
            &[escrow.vault_bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.seller_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            ),
            escrow.amount,
        )?;
        close_vault(
            &ctx.accounts.vault,
            &ctx.accounts.buyer,
            &ctx.accounts.vault_authority,
            &ctx.accounts.token_program,
            signer,
        )?;
        escrow.status = EscrowStatus::Released;
        emit!(EscrowReleased {
            order_id: escrow.order_id,
            escrow: escrow.key(),
            recipient: escrow.seller,
            amount: escrow.amount
        });
        Ok(())
    }
    pub fn refund_escrow(ctx: Context<RefundEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Funded,
            EscrowError::InvalidStatus
        );
        require_keys_eq!(
            ctx.accounts.buyer.key(),
            escrow.buyer,
            EscrowError::Unauthorized
        );
        require!(
            Clock::get()?.unix_timestamp >= escrow.deadline,
            EscrowError::RefundTooEarly
        );
        require_keys_eq!(
            ctx.accounts.buyer_token_account.owner,
            escrow.buyer,
            EscrowError::InvalidTokenAccount
        );
        require_keys_eq!(
            ctx.accounts.buyer_token_account.mint,
            escrow.mint,
            EscrowError::WrongMint
        );
        let escrow_key = escrow.key();
        let seeds = &[
            b"vault-authority",
            escrow_key.as_ref(),
            &[escrow.vault_bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.buyer_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            ),
            escrow.amount,
        )?;
        close_vault(
            &ctx.accounts.vault,
            &ctx.accounts.buyer,
            &ctx.accounts.vault_authority,
            &ctx.accounts.token_program,
            signer,
        )?;
        escrow.status = EscrowStatus::Refunded;
        emit!(EscrowRefunded {
            order_id: escrow.order_id,
            escrow: escrow.key(),
            recipient: escrow.buyer,
            amount: escrow.amount
        });
        Ok(())
    }
    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        require!(
            escrow.status == EscrowStatus::Created,
            EscrowError::InvalidStatus
        );
        require_keys_eq!(
            ctx.accounts.buyer.key(),
            escrow.buyer,
            EscrowError::Unauthorized
        );
        escrow.status = EscrowStatus::Cancelled;
        emit!(EscrowCancelled {
            order_id: escrow.order_id,
            escrow: escrow.key()
        });
        Ok(())
    }
}

fn close_vault<'info>(
    vault: &Account<'info, TokenAccount>,
    destination: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    signer: &[&[&[u8]]],
) -> Result<()> {
    let accounts = CloseAccount {
        account: vault.to_account_info(),
        destination: destination.clone(),
        authority: authority.clone(),
    };
    token::close_account(CpiContext::new_with_signer(
        token_program.to_account_info(),
        accounts,
        signer,
    ))
}

#[derive(Accounts)]
#[instruction(order_id: [u8; 32])]
pub struct InitializeEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: seller is a stored recipient; token-account ownership is checked at settlement.
    pub seller: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    #[account(init, payer = buyer, space = 8 + Escrow::SIZE, seeds = [b"escrow", buyer.key().as_ref(), order_id.as_ref()], bump)]
    pub escrow: Account<'info, Escrow>,
    /// CHECK: PDA authority validated by seeds.
    #[account(seeds = [b"vault-authority", escrow.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct FundEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = buyer, seeds = [b"escrow", buyer.key().as_ref(), escrow.order_id.as_ref()], bump = escrow.bump)]
    pub escrow: Account<'info, Escrow>,
    pub mint: Account<'info, Mint>,
    #[account(mut, constraint = buyer_token_account.owner == buyer.key() @ EscrowError::InvalidTokenAccount, constraint = buyer_token_account.mint == escrow.mint @ EscrowError::WrongMint)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(init, payer = buyer, token::mint = mint, token::authority = vault_authority, seeds = [b"vault", escrow.key().as_ref()], bump)]
    pub vault: Account<'info, TokenAccount>,
    /// CHECK: PDA authority validated by seeds.
    #[account(seeds = [b"vault-authority", escrow.key().as_ref()], bump = escrow.vault_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
pub struct ReleaseEscrow<'info> {
    pub release_authority: Signer<'info>,
    /// CHECK: constrained to the immutable buyer stored in the escrow.
    #[account(mut, address = escrow.buyer)]
    pub buyer: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"escrow", escrow.buyer.as_ref(), escrow.order_id.as_ref()], bump = escrow.bump)]
    pub escrow: Account<'info, Escrow>,
    /// CHECK: PDA authority validated by seeds.
    #[account(seeds = [b"vault-authority", escrow.key().as_ref()], bump = escrow.vault_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump, constraint = vault.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority, constraint = vault.mint == escrow.mint @ EscrowError::WrongMint)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub seller_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct RefundEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = buyer, seeds = [b"escrow", buyer.key().as_ref(), escrow.order_id.as_ref()], bump = escrow.bump)]
    pub escrow: Account<'info, Escrow>,
    /// CHECK: PDA authority validated by seeds.
    #[account(seeds = [b"vault-authority", escrow.key().as_ref()], bump = escrow.vault_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump, constraint = vault.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority, constraint = vault.mint == escrow.mint @ EscrowError::WrongMint)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, constraint = buyer_token_account.owner == buyer.key() @ EscrowError::InvalidTokenAccount, constraint = buyer_token_account.mint == escrow.mint @ EscrowError::WrongMint)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}
#[derive(Accounts)]
pub struct CancelEscrow<'info> {
    pub buyer: Signer<'info>,
    #[account(mut, close = buyer, has_one = buyer, seeds = [b"escrow", buyer.key().as_ref(), escrow.order_id.as_ref()], bump = escrow.bump)]
    pub escrow: Account<'info, Escrow>,
}

#[account]
pub struct Escrow {
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub deadline: i64,
    pub order_id: [u8; 32],
    pub status: EscrowStatus,
    pub bump: u8,
    pub vault_bump: u8,
    pub release_arbiter: Pubkey,
    pub bump_version: u8,
}
impl Escrow {
    pub const SIZE: usize = 32 + 32 + 32 + 8 + 8 + 32 + 1 + 1 + 1 + 32 + 1;
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum EscrowStatus {
    Created,
    Funded,
    Released,
    Refunded,
    Cancelled,
}
#[event]
pub struct EscrowCreated {
    pub order_id: [u8; 32],
    pub escrow: Pubkey,
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub amount: u64,
    pub deadline: i64,
    pub mint: Pubkey,
}
#[event]
pub struct EscrowFunded {
    pub order_id: [u8; 32],
    pub escrow: Pubkey,
    pub amount: u64,
    pub vault: Pubkey,
}
#[event]
pub struct EscrowReleased {
    pub order_id: [u8; 32],
    pub escrow: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
}
#[event]
pub struct EscrowRefunded {
    pub order_id: [u8; 32],
    pub escrow: Pubkey,
    pub recipient: Pubkey,
    pub amount: u64,
}
#[event]
pub struct EscrowCancelled {
    pub order_id: [u8; 32],
    pub escrow: Pubkey,
}
#[error_code]
pub enum EscrowError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
    #[msg("Deadline must be in the future")]
    DeadlineInPast,
    #[msg("Deadline is too far in the future")]
    DeadlineTooFar,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Unauthorized signer")]
    Unauthorized,
    #[msg("Invalid escrow status")]
    InvalidStatus,
    #[msg("Token mint does not match escrow")]
    WrongMint,
    #[msg("Vault token authority does not match the escrow PDA authority")]
    InvalidVaultAuthority,
    #[msg("Invalid token account owner")]
    InvalidTokenAccount,
    #[msg("Refund deadline has not passed")]
    RefundTooEarly,
}
