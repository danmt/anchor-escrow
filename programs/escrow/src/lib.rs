use anchor_lang::prelude::*;

declare_id!("8cfi1HEe8ZVdCZhUku2RxLwhQat3BgY8otHB3PmHGd4y");

#[program]
pub mod escrow {
    use super::*;

    pub fn start_trade(
        ctx: Context<StartTrade>,
        amount_offered: u64,
        amount_requested: u64,
    ) -> Result<()> {
        ctx.accounts.trade.finalized = false;
        ctx.accounts.trade.amount_offered = amount_offered;
        ctx.accounts.trade.amount_requested = amount_requested;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(amount_offered: u64, amount_requested: u64)]
pub struct StartTrade<'info> {
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 200,
    )]
    pub trade: Account<'info, Trade>,
}

#[account]
pub struct Trade {
    // Cantidad que el solicitante desea
    amount_requested: u64,
    mint_requested: Pubkey,

    // Cantidad que el solicitante ofrece
    amount_offered: u64,
    mint_offered: Pubkey,

    finalized: bool,
}
