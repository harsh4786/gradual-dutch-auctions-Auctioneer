use anchor_lang::{prelude::*,
    AnchorDeserialize,
    InstructionData,
    solana_program::{self, clock::UnixTimestamp, 
    system_instruction, 
    program::{invoke, invoke_signed},
    program_memory::sol_memset,
   
}};
pub mod util;
use crate::util::{assert_keys_equal, assert_auction_active, assert_valid_auctioneer};
pub mod errors;
use crate::errors::*;

use anchor_spl::token::{Token, Mint, TokenAccount};
use mpl_auction_house::{
    self,
    constants::{AUCTIONEER, FEE_PAYER, PREFIX, SIGNER, TRADE_STATE_SIZE},
    program::AuctionHouse as AuctionHouseProgram,
    cpi::accounts::AuctioneerSell as AHSell,
    AuctionHouse, errors::AuctionHouseError,
    Auctioneer,
    utils::*,
};
pub mod math;
use math::*;
pub const LISTING_CONFIG: &str = "listing_config";
pub const AUCTIONEER_BUYER_PRICE: u64 = u64::MAX;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");


#[program]
pub mod gda {
    use super::*;
    pub fn auctioneer_sell<'info>(
        ctx: Context<'_, '_, '_, 'info, AuctioneerSell<'info>>,
        trade_state_bump: u8,
        free_trade_state_bump: u8,
        program_as_signer_bump: u8,
        auctioneer_authority_bump: u8,
        token_size: u64,
        //start_time: UnixTimestamp,
        end_time: UnixTimestamp,
        start_price: u64,
        decay_constant: u8,
        scale_factor: u64,
        
    ) -> Result<()> {
        ctx.accounts.listing_config.token_size = token_size;
        ctx.accounts.listing_config.first_init_timestamp = Clock::get()?.unix_timestamp;
        ctx.accounts.listing_config.end_timestamp = end_time;
        ctx.accounts.listing_config.start_price = start_price;
        ctx.accounts.listing_config.decay_const = decay_constant; 
        ctx.accounts.listing_config.scale_factor = scale_factor;
        ctx.accounts.listing_config.items_sold = 0;
        ctx.accounts.listing_config.bump = *ctx
            .bumps
            .get("listing_config")
            .ok_or(AuctioneerError::BumpSeedNotInHashMap)?;
    
        let cpi_program = ctx.accounts.auction_house_program.to_account_info();
        let cpi_accounts = AHSell {
            wallet: ctx.accounts.wallet.to_account_info(),
            token_account: ctx.accounts.token_account.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            auction_house: ctx.accounts.auction_house.to_account_info(),
            auction_house_fee_account: ctx.accounts.auction_house_fee_account.to_account_info(),
            seller_trade_state: ctx.accounts.seller_trade_state.to_account_info(),
            free_seller_trade_state: ctx.accounts.free_seller_trade_state.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
            auctioneer_authority: ctx.accounts.auctioneer_authority.to_account_info(),
            ah_auctioneer_pda: ctx.accounts.ah_auctioneer_pda.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            program_as_signer: ctx.accounts.program_as_signer.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };
    
        let sell_data = mpl_auction_house::instruction::AuctioneerSell {
            trade_state_bump,
            free_trade_state_bump,
            program_as_signer_bump,
            token_size,
        };
    
        let ix = solana_program::instruction::Instruction {
            program_id: cpi_program.key(),
            accounts: cpi_accounts
                .to_account_metas(None)
                .into_iter()
                .zip(cpi_accounts.to_account_infos())
                .map(|mut pair| {
                    pair.0.is_signer = pair.1.is_signer;
                    if pair.0.pubkey == ctx.accounts.auctioneer_authority.key() {
                        pair.0.is_signer = true;
                    }
                    pair.0
                })
                .collect(),
            data: sell_data.data(),
        };
    
        let auction_house = &ctx.accounts.auction_house;
        let ah_key = auction_house.key();
        let auctioneer_authority = &ctx.accounts.auctioneer_authority;
        let _aa_key = auctioneer_authority.key();
    
        let auctioneer_seeds = [
            AUCTIONEER.as_bytes(),
            ah_key.as_ref(),
            &[auctioneer_authority_bump],
        ];
    
        invoke_signed(&ix, &cpi_accounts.to_account_infos(), &[&auctioneer_seeds])?;
    
        Ok(())
    }

    pub fn place_order<'info>(
        ctx: Context<'_,'_,'_,'info, AuctioneerBuy<'info>>,
        order_size: u64,
        trade_state_bump: u8,
        escrow_payment_bump: u8,
        //auctioneer_authority_bump: u8,
    ) -> Result<()> {
        assert_auction_active(&ctx.accounts.listing_config)?;
        let cumulative_price = &ctx.accounts.listing_config.calculate_price(order_size).unwrap();
        auctioneer_place_order_logic(
            ctx.accounts.wallet.to_owned(),
            ctx.accounts.payment_account.to_owned(),
            ctx.accounts.transfer_authority.to_owned(),
            *ctx.accounts.treasury_mint.to_owned(),
            *ctx.accounts.token_account.to_owned(),
            ctx.accounts.metadata.to_owned(),
            ctx.accounts.escrow_payment_account.to_owned(),
            &mut ctx.accounts.auction_house,
            ctx.accounts.auction_house_fee_account.to_owned(),
            ctx.accounts.buyer_trade_state.to_owned(),
            ctx.accounts.authority.to_owned(),
            ctx.accounts.auctioneer_authority.to_owned(),
            ctx.accounts.ah_auctioneer_pda.to_owned(),
            ctx.accounts.token_program.to_owned(),
            ctx.accounts.system_program.to_owned(),
            ctx.accounts.rent.to_owned(),
            trade_state_bump,
            escrow_payment_bump,
            *cumulative_price,
            order_size,
            false,
            *ctx.bumps
                .get("escrow_payment_account")
                .ok_or(AuctionHouseError::BumpSeedNotInHashMap)?,
            *ctx.bumps
                .get("buyer_trade_state")
                .ok_or(AuctionHouseError::BumpSeedNotInHashMap)?,
        )?;
        Ok(())
    }
   
}

#[derive(Accounts)]
#[instruction(trade_state_bump: u8, free_trade_state_bump: u8, program_as_signer_bump: u8, auctioneer_authority_bump: u8, token_size: u64)]
pub struct AuctioneerSell<'info>{
    pub auction_house_program: Program<'info, AuctionHouseProgram>,

    #[account(
        init,
        payer=wallet,
        space= std::mem::size_of::<ListingConfig>(),
        seeds=[
            LISTING_CONFIG.as_bytes(),
            wallet.key().as_ref(),
            auction_house.key().as_ref(),
            token_account.key().as_ref(),
            auction_house.treasury_mint.as_ref(),
            token_account.mint.as_ref(),
            &token_size.to_le_bytes(), 
        ],
        bump,
    )]
    pub listing_config: Account<'info, ListingConfig>,

    /// SPL token account containing token for sale.
    #[account(mut)]
    pub token_account: Box<Account<'info, TokenAccount>>,
    
    pub metadata: UncheckedAccount<'info>,

    /// CHECK: Verified through CPI
    /// Auction House authority account.
    pub authority: UncheckedAccount<'info>,

    #[account(seeds=[PREFIX.as_bytes(), auction_house.creator.as_ref(), auction_house.treasury_mint.as_ref()], seeds::program=auction_house_program, bump=auction_house.bump, has_one=auction_house_fee_account)]
    pub auction_house: Box<Account<'info, AuctionHouse>>,

    #[account(mut, seeds=[PREFIX.as_bytes(), auction_house.key().as_ref(), FEE_PAYER.as_bytes()], seeds::program=auction_house_program, bump=auction_house.fee_payer_bump)]
    pub auction_house_fee_account: UncheckedAccount<'info>,

    
    #[account(mut, seeds=[PREFIX.as_bytes(), wallet.key().as_ref(), auction_house.key().as_ref(), token_account.key().as_ref(), auction_house.treasury_mint.as_ref(), token_account.mint.as_ref(), &u64::MAX.to_le_bytes(), &token_size.to_le_bytes()], seeds::program=auction_house_program, bump=trade_state_bump)]
    pub seller_trade_state: UncheckedAccount<'info>,

    #[account(mut, seeds=[PREFIX.as_bytes(), wallet.key().as_ref(), auction_house.key().as_ref(), token_account.key().as_ref(), auction_house.treasury_mint.as_ref(), token_account.mint.as_ref(), &0u64.to_le_bytes(), &token_size.to_le_bytes()], seeds::program=auction_house_program, bump=free_trade_state_bump)]
    pub free_seller_trade_state: UncheckedAccount<'info>,

    /// CHECK: Verified through CPI
    /// The auctioneer program PDA running this auction.
    /// this is the wallet owner that is the authority of the main auction house setup
    pub auctioneer_authority: Signer<'info>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    /// The auctioneer PDA owned by Auction House storing scopes.
    #[account(
        seeds = [
            AUCTIONEER.as_bytes(),
            auction_house.key().as_ref(),
            auctioneer_authority.key().as_ref()
            ],
        seeds::program=auction_house_program,
        bump = ah_auctioneer_pda.bump,
    )]
    pub ah_auctioneer_pda: Account<'info, mpl_auction_house::Auctioneer>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    #[account(seeds=[PREFIX.as_bytes(), SIGNER.as_bytes()], seeds::program=auction_house_program, bump=program_as_signer_bump)]
    pub program_as_signer: UncheckedAccount<'info>,

    #[account(mut)]
    pub wallet: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
#[instruction(trade_state_bump: u8, escrow_payment_bump: u8, auctioneer_authority_bump: u8, token_size: u64)]
pub struct AuctioneerBuy<'info> {
    /// Auction House Program
    pub auction_house_program: Program<'info, AuctionHouseProgram>,

    // Accounts used for Auctioneer
    /// The Listing Config used for listing settings
    #[account(
        mut,
        seeds=[
            LISTING_CONFIG.as_bytes(),
            seller.key().as_ref(),
            auction_house.key().as_ref(),
            token_account.key().as_ref(),
            auction_house.treasury_mint.as_ref(),
            token_account.mint.as_ref(),
            &token_size.to_le_bytes()
        ],
        bump,
    )]
    pub listing_config: Account<'info, ListingConfig>,

    /// The seller of the NFT
    /// CHECK: Checked via trade state constraints
    pub seller: UncheckedAccount<'info>,

    // Accounts passed into Auction House CPI call
    /// User wallet account.
    wallet: Signer<'info>,

    /// CHECK: Verified through CPI
    /// User SOL or SPL account to transfer funds from.
    #[account(mut)]
    payment_account: UncheckedAccount<'info>,

    /// CHECK:
    /// SPL token account transfer authority.
    transfer_authority: UncheckedAccount<'info>,

    /// Auction House instance treasury mint account.
    treasury_mint: Box<Account<'info, Mint>>,

    /// SPL token account.
    token_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: Verified through CPI
    /// SPL token account metadata.
    metadata: UncheckedAccount<'info>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    /// Buyer escrow payment account PDA.
    #[account(
        mut,
        seeds = [
            PREFIX.as_bytes(),
            auction_house.key().as_ref(),
            wallet.key().as_ref()
        ], seeds::program=auction_house_program,
        bump = escrow_payment_bump,
    )]
    escrow_payment_account: UncheckedAccount<'info>,

    /// CHECK: Verified with has_one constraint on auction house account.
    /// Auction House instance authority account.
    authority: UncheckedAccount<'info>,

    /// Auction House instance PDA account.
    #[account(seeds = [PREFIX.as_bytes(), auction_house.creator.as_ref(), auction_house.treasury_mint.as_ref()], seeds::program=auction_house_program, bump = auction_house.bump, has_one = authority, has_one = treasury_mint, has_one = auction_house_fee_account)]
    auction_house: Box<Account<'info, AuctionHouse>>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    /// Auction House instance fee account.
    #[account(mut, seeds = [PREFIX.as_bytes(), auction_house.key().as_ref(), FEE_PAYER.as_bytes()], seeds::program=auction_house_program, bump = auction_house.fee_payer_bump)]
    auction_house_fee_account: UncheckedAccount<'info>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    /// Buyer trade state PDA.
    ///
      
    #[account(mut, seeds = [PREFIX.as_bytes(), wallet.key().as_ref(), auction_house.key().as_ref(), token_account.key().as_ref(), treasury_mint.key().as_ref(), token_account.mint.as_ref(), /*buyer_price.to_le_bytes().as_ref()*/ token_size.to_le_bytes().as_ref()], seeds::program=auction_house_program, bump = trade_state_bump)]
    buyer_trade_state: UncheckedAccount<'info>,
    
    /// CHECK: Is used as a seed for ah_auctioneer_pda.
    /// The auctioneer program PDA running this auction.
    pub auctioneer_authority: Signer<'info>,

    /// CHECK: Not dangerous. Account seeds checked in constraint.
    /// The auctioneer PDA owned by Auction House storing scopes.
    #[account(
        seeds = [
            AUCTIONEER.as_bytes(),
            auction_house.key().as_ref(),
            auctioneer_authority.key().as_ref()
        ], seeds::program=auction_house_program,
        bump = ah_auctioneer_pda.bump,
    )]
    pub ah_auctioneer_pda: Account<'info, mpl_auction_house::Auctioneer>,

    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}



#[account]
pub struct ListingConfig{
    pub token_size: u64, //Total items to be sold in that auction
    pub items_sold: u64, //'m' value - cumulative of token sizes being sold (if it's the first sale of the auction) or already sold
    pub start_price: u64, //  'k' in the paradigm's equation
    pub decay_const: u8, // lambda 
    pub scale_factor: u64, // alpha responsible for the increase in the initial start_price
    pub first_init_timestamp: UnixTimestamp, // timestamp of the genesis of the first auction
    pub end_timestamp: UnixTimestamp, //
    //pub last_updated_ts: UnixTimestamp, //
    //pub auction_interval: u64,
    //pub auction_index: u64, // index
    pub bump: u8,
}
impl ListingConfig{
    pub fn calculate_price(&self, order_size: u64) -> Result<u64>{
        let m = Decimal::from_integer(self.items_sold);
        let k = Decimal::from_integer(self.start_price);
        let q = Decimal::from_integer(order_size);
        let e = Decimal::euler_value();
        let one = Decimal::from_integer(1);
        let decay = self.decay_const as u128;
        let now = Clock::get()?.unix_timestamp as u64;
        let t = Decimal::from_integer(now.checked_sub(self.first_init_timestamp as u64).unwrap());
        let a = Decimal::from_integer(self.scale_factor);
        let num1 = k.mul(a.pow_with_accuracy(m.val));
        let num2 = a.pow_with_accuracy(q.val).sub(one).unwrap();
        let den1 = e.pow_with_accuracy(t.mul(decay).val);
        let den2 = a.sub(one).unwrap();
        let num = num1.mul(num2);
        let den = den1.mul(den2);
        let cumulative_price = num.div_up(den).to_scale(0).val as u64;
        Ok(cumulative_price)
    }
}


#[allow(clippy::too_many_arguments)]
pub fn auctioneer_place_order_logic<'info>(
    wallet: Signer<'info>,
    payment_account: UncheckedAccount<'info>,
    transfer_authority: UncheckedAccount<'info>,
    treasury_mint: Account<'info, Mint>,
    token_account: Account<'info, TokenAccount>,
    metadata: UncheckedAccount<'info>,
    escrow_payment_account: UncheckedAccount<'info>,
    auction_house: &mut Box<Account<'info, AuctionHouse>>,
    auction_house_fee_account: UncheckedAccount<'info>,
    buyer_trade_state: UncheckedAccount<'info>,
    authority: UncheckedAccount<'info>,
    auctioneer_authority: Signer<'info>,
    ah_auctioneer_pda: Account<'info, Auctioneer>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    trade_state_bump: u8,
    escrow_payment_bump: u8,
    buyer_price: u64,
    token_size: u64,
    public: bool,
    escrow_canonical_bump: u8,
    trade_state_canonical_bump: u8,
) -> Result<()>{

    if !auction_house.has_auctioneer {
        return Err(AuctionHouseError::NoAuctioneerProgramSet.into());
    }

    assert_valid_auctioneer(
        auction_house,
        &auctioneer_authority.key,
        &ah_auctioneer_pda,
    )?;
    if (escrow_canonical_bump != escrow_payment_bump)
        || (trade_state_canonical_bump != trade_state_bump)
    {
        return Err(AuctionHouseError::BumpSeedNotInHashMap.into());
    }

    let auction_house_key = auction_house.key();
    let seeds = [
        PREFIX.as_bytes(),
        auction_house_key.as_ref(),
        FEE_PAYER.as_bytes(),
        &[auction_house.fee_payer_bump],
    ];

    // makes a fee payer account depending on whether or not if the buyer (bidder)
    // has signed to pay the fees or the auction house's authority has signed 
    // to pay the fees, if the buyer signs it then the auction house authority will
    // need to approve or sign to set the fee payer to the buyer.
    // note that the above is true only if auction house's "requires_sign_off" field
    // is set to true.
    let (fee_payer, fee_seeds) = get_fee_payer(
        &authority, // auction house authority
        auction_house,
        wallet.to_account_info(), //buyer or bidder's wallet
        auction_house_fee_account.to_account_info(),
        &seeds,
    )?;

    let is_native = treasury_mint.key() == spl_token::native_mint::id();

    let auction_house_key = auction_house.key();
    let wallet_key = wallet.key();
    let escrow_signer_seeds = [
        PREFIX.as_bytes(),
        auction_house_key.as_ref(),
        wallet_key.as_ref(),
        &[escrow_payment_bump],
    ];

    //this will create the escrow_payment_account if it's not already created.
    create_program_token_account_if_not_present(
        &escrow_payment_account,
        &system_program,
        &fee_payer,
        &token_program,
        &treasury_mint,
        &auction_house.to_account_info(),
        &rent,
        &escrow_signer_seeds,
        fee_seeds,
        is_native,
    )?;
    
    if is_native {
        assert_keys_equal(wallet.key(), payment_account.key())?;

        if escrow_payment_account.lamports()
            < buyer_price
                .checked_add(rent.minimum_balance(escrow_payment_account.data_len()))
                .ok_or(AuctionHouseError::NumericalOverflow)?
        {
            let diff = buyer_price
                .checked_add(rent.minimum_balance(escrow_payment_account.data_len()))
                .ok_or(AuctionHouseError::NumericalOverflow)?
                .checked_sub(escrow_payment_account.lamports())
                .ok_or(AuctionHouseError::NumericalOverflow)?;

            invoke(
                &system_instruction::transfer(
                    &payment_account.key(),
                    &escrow_payment_account.key(),
                    diff,
                ),
                &[
                    payment_account.to_account_info(),
                    escrow_payment_account.to_account_info(),
                    system_program.to_account_info(),
                ],
            )?;
        }
    } else {
        let escrow_payment_loaded: spl_token::state::Account =
            assert_initialized(&escrow_payment_account)?;

        if escrow_payment_loaded.amount < buyer_price {
            let diff = buyer_price
                .checked_sub(escrow_payment_loaded.amount)
                .ok_or(AuctionHouseError::NumericalOverflow)?;
            invoke(
                &spl_token::instruction::transfer(
                    &token_program.key(),
                    &payment_account.key(),
                    &escrow_payment_account.key(),
                    &transfer_authority.key(),
                    &[],
                    diff,
                )?,
                &[
                    transfer_authority.to_account_info(),
                    payment_account.to_account_info(),
                    escrow_payment_account.to_account_info(),
                    token_program.to_account_info(),
                ],
            )?;
        }
    }
    assert_metadata_valid(&metadata, &token_account)?;
    let ts_info = buyer_trade_state.to_account_info();
    if ts_info.data_is_empty() {
        let wallet_key = wallet.key();
        let token_account_key = token_account.key();
        if public {
            create_or_allocate_account_raw(
                crate::id(),
                &ts_info,
                &rent.to_account_info(),
                &system_program,
                &fee_payer,
                TRADE_STATE_SIZE,
                fee_seeds,
                &[
                    PREFIX.as_bytes(),
                    wallet_key.as_ref(),
                    auction_house_key.as_ref(),
                    auction_house.treasury_mint.as_ref(),
                    token_account.mint.as_ref(),
                    &buyer_price.to_le_bytes(),
                    &token_size.to_le_bytes(),
                    &[trade_state_bump],
                ],
            )?;
        } else {
            create_or_allocate_account_raw(
                crate::id(),
                &ts_info,
                &rent.to_account_info(),
                &system_program,
                &fee_payer,
                TRADE_STATE_SIZE,
                fee_seeds,
                &[
                    PREFIX.as_bytes(),
                    wallet_key.as_ref(),
                    auction_house_key.as_ref(),
                    token_account_key.as_ref(),
                    auction_house.treasury_mint.as_ref(),
                    token_account.mint.as_ref(),
                    &buyer_price.to_le_bytes(),
                    &token_size.to_le_bytes(),
                    &[trade_state_bump],
                ],
            )?;
        }
        sol_memset(
            *ts_info.try_borrow_mut_data()?,
            trade_state_bump,
            TRADE_STATE_SIZE,
        );
    }
    // Allow The same bid to be sent with no issues
    Ok(())
}