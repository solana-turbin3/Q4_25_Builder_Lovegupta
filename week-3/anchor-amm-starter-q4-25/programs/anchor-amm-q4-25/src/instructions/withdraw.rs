use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
    mut,
    seeds=[b"config",config.seed.to_le_bytes().as_ref()],
    bump=config.config_bump,
 )]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint=mint_lp,
        associated_token::authority=user
    )]
    pub user_lp:Account<'info,TokenAccount>,
    #[account(
    mut,
    associated_token::mint=mint_x,
    associated_token::authority=config
  )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
    mut,
    associated_token::mint=mint_y,
    associated_token::authority=config
  )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
    mut,
    associated_token::mint=mint_x,
    associated_token::authority=user
  )]
    pub user_x: Account<'info, TokenAccount>,

    #[account(
    mut,
    associated_token::mint=mint_y,
    associated_token::authority=user
  )]
    pub user_y: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        amount: u64, // Amount of LP tokens that the user wants to "burn"
        min_x: u64,  // Minimum amount of token X that the user wants to receive
        min_y: u64,  // Minimum amount of token Y that the user wants to receive
    ) -> Result<()> {

        require!(self.config.locked==false,AmmError::PoolLocked);
        require!(amount!=0,AmmError::InvalidAmount);
        require!(self.mint_lp.supply!=0,AmmError::InsufficientBalance);

       let mut c=ConstantProduct::init(self.vault_x.amount, self.vault_y.amount,self.mint_lp.supply,self.config.fee,Some(6)).unwrap();

       let withdraw_res=c.withdraw_liquidity(amount, min_x, min_y).unwrap();

    
      self.withdraw_tokens(true, withdraw_res.withdraw_x)?;
      self.withdraw_tokens(false, withdraw_res.withdraw_y)?;
      self.burn_lp_tokens(withdraw_res.burn_l)
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
            false => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
        };

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.config.to_account_info(),
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];

        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );

        transfer(cpi_ctx, amount)
    }

    pub fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        
        let cpi_accounts=Burn{
        mint:self.mint_lp.to_account_info(),
        from:self.user_lp.to_account_info(),
        authority:self.user.to_account_info()
        };
        // let signer_seeds:&[&[&[u8]]]=&[&[b"config",&self.config.seed.to_be_bytes(),&[self.config.config_bump]]];
       let cpi_ctx=CpiContext::new(self.token_program.to_account_info(), cpi_accounts);

       burn(cpi_ctx, amount)
    }
}
