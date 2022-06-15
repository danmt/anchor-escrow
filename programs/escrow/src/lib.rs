use anchor_lang::prelude::*;
use anchor_spl::token::*;

declare_id!("8cfi1HEe8ZVdCZhUku2RxLwhQat3BgY8otHB3PmHGd4y");

#[program]
pub mod escrow {
    use super::*;

    pub fn start_trade(
        ctx: Context<StartTrade>,
        amount_offered: u64,
        amount_requested: u64,
    ) -> Result<()> {
        (*ctx.accounts.trade).executed = false;
        (*ctx.accounts.trade).amount_offered = amount_offered;
        (*ctx.accounts.trade).amount_requested = amount_requested;
        (*ctx.accounts.trade).author = ctx.accounts.authority.key();
        (*ctx.accounts.trade).mint_offered = ctx.accounts.mint_offered.key();
        (*ctx.accounts.trade).mint_requested = ctx.accounts.mint_requested.key();
        (*ctx.accounts.trade).bump = *ctx.bumps.get("trade").unwrap();
        (*ctx.accounts.trade).vault_bump = *ctx.bumps.get("trade_vault").unwrap();

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.author_vault.to_account_info(),
                    to: ctx.accounts.trade_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount_offered,
        )?;

        Ok(())
    }

    pub fn cancel_trade(ctx: Context<CancelTrade>) -> Result<()> {
        let trade_seeds = &[
            b"trade".as_ref(),
            ctx.accounts.base.to_account_info().key.as_ref(),
            &[ctx.accounts.trade.bump],
        ];

        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trade_vault.to_account_info(),
                    to: ctx.accounts.author_vault.to_account_info(),
                    authority: ctx.accounts.trade.to_account_info(),
                },
                &[&trade_seeds[..]],
            ),
            ctx.accounts.trade.amount_offered,
        )?;

        anchor_spl::token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.trade_vault.to_account_info(),
                destination: ctx.accounts.authority.to_account_info(),
                authority: ctx.accounts.trade.to_account_info(),
            },
            &[&trade_seeds[..]],
        ))?;

        Ok(())
    }

    pub fn execute_trade(ctx: Context<ExecuteTrade>) -> Result<()> {
        let trade_seeds = &[
            b"trade".as_ref(),
            ctx.accounts.base.to_account_info().key.as_ref(),
            &[ctx.accounts.trade.bump],
        ];

        // Transfer from trade vault to executer vault
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.trade_vault.to_account_info(),
                    to: ctx.accounts.executer_offered_vault.to_account_info(),
                    authority: ctx.accounts.trade.to_account_info(),
                },
                &[&trade_seeds[..]],
            ),
            ctx.accounts.trade.amount_offered,
        )?;

        // Transfer from executer vault to author vault
        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.executer_requested_vault.to_account_info(),
                    to: ctx.accounts.author_requested_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            ctx.accounts.trade.amount_requested,
        )?;

        (*ctx.accounts.trade).executed = true;

        Ok(())
    }

    pub fn delete_trade(ctx: Context<DeleteTrade>) -> Result<()> {
        let trade_seeds = &[
            b"trade".as_ref(),
            ctx.accounts.base.to_account_info().key.as_ref(),
            &[ctx.accounts.trade.bump],
        ];

        anchor_spl::token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.trade_vault.to_account_info(),
                destination: ctx.accounts.authority.to_account_info(),
                authority: ctx.accounts.trade.to_account_info(),
            },
            &[&trade_seeds[..]],
        ))?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(amount_offered: u64, amount_requested: u64)]
pub struct StartTrade<'info> {
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Base is an arbitrary Pubkey used to generate the trade
    pub base: UncheckedAccount<'info>,
    #[account(
        init,
        payer = authority,
        space = 200,
        seeds = [
            b"trade".as_ref(),
            base.key().as_ref(),
        ],
        bump,
    )]
    pub trade: Account<'info, Trade>,
    pub mint_offered: Account<'info, Mint>,
    pub mint_requested: Account<'info, Mint>,
    #[account(
        mut,
        constraint = author_vault.mint == mint_offered.key()
    )]
    pub author_vault: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = authority,
        seeds = [
            b"trade_vault".as_ref(),
            trade.key().as_ref(),
        ],
        bump,
        token::mint = mint_offered,
        token::authority = trade,
    )]
    pub trade_vault: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct CancelTrade<'info> {
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Base is an arbitrary Pubkey used to generate the trade
    pub base: UncheckedAccount<'info>,
    #[account(
        mut,
        close = authority,
        constraint = trade.author == authority.key(),
        seeds = [
            b"trade".as_ref(),
            base.key().as_ref(),
        ],
        bump = trade.bump,
    )]
    pub trade: Account<'info, Trade>,
    #[account(
        mut,
        seeds = [
            b"trade_vault".as_ref(),
            trade.key().as_ref(),
        ],
        bump,
    )]
    pub trade_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = author_vault.mint == trade.mint_offered
    )]
    pub author_vault: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct ExecuteTrade<'info> {
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Base is an arbitrary Pubkey used to generate the trade
    pub base: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = !trade.executed,
        seeds = [
            b"trade".as_ref(),
            base.key().as_ref(),
        ],
        bump = trade.bump,
    )]
    pub trade: Account<'info, Trade>,
    #[account(
        mut,
        seeds = [
            b"trade_vault".as_ref(),
            trade.key().as_ref(),
        ],
        bump,
    )]
    pub trade_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = author_requested_vault.mint == trade.mint_requested
    )]
    pub author_requested_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = executer_offered_vault.mint == trade.mint_offered
    )]
    pub executer_offered_vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = executer_requested_vault.mint == trade.mint_requested
    )]
    pub executer_requested_vault: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct DeleteTrade<'info> {
    pub token_program: Program<'info, Token>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Base is an arbitrary Pubkey used to generate the trade
    pub base: UncheckedAccount<'info>,
    #[account(
        mut, 
        close = authority, 
        constraint = trade.author == authority.key(),
        seeds = [
            b"trade".as_ref(),
            base.key().as_ref(),
        ],
        bump = trade.bump,
    )]
    pub trade: Account<'info, Trade>,
    #[account(
        mut,
        seeds = [
            b"trade_vault".as_ref(),
            trade.key().as_ref(),
        ],
        bump,
    )]
    pub trade_vault: Account<'info, TokenAccount>,
}

#[account]
pub struct Trade {
    author: Pubkey,
    executed: bool,
    amount_requested: u64,
    mint_requested: Pubkey,
    amount_offered: u64,
    mint_offered: Pubkey,
    bump: u8,
    vault_bump: u8,
}
