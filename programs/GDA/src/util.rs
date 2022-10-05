use anchor_lang::{
    prelude::*,
    solana_program::{
       // program::invoke_signed,
        program_memory::sol_memcmp,
       // program_option::COption,
      //  program_pack::{IsInitialized, Pack},
        pubkey::PUBKEY_BYTES,
        //system_instruction,
    },
};

use crate::{errors::*, ListingConfig, AuctionHouse,  Auctioneer,AuctionHouseError };

pub fn assert_auction_active(listing_config: &Account<ListingConfig>) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    if current_timestamp < listing_config.first_init_timestamp {
        return err!(AuctioneerError::AuctionNotStarted);
    } else if current_timestamp > listing_config.end_timestamp || listing_config.items_sold == listing_config.token_size {
        return err!(AuctioneerError::AuctionEnded);
    }

    Ok(())
}

pub fn assert_auction_over(listing_config: &Account<ListingConfig>) -> Result<()> {
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;

    if current_timestamp < listing_config.end_timestamp {
        return err!(AuctioneerError::AuctionActive);
    }

    Ok(())
}


pub fn assert_valid_auctioneer(
    auction_house_instance: &Account<AuctionHouse>,
    auctioneer_authority: &Pubkey,
    auctioneer_pda: &Account<Auctioneer>,
) -> Result<()>{
    assert_keys_equal(
        auction_house_instance.auctioneer_address,
        auctioneer_pda.key(),
    )
    .map_err(|_e| AuctionHouseError::InvalidAuctioneer)?;
    assert_keys_equal(
        auctioneer_pda.auctioneer_authority,
        auctioneer_authority.key(),
    )
    .map_err(|_e| AuctionHouseError::InvalidAuctioneer)?;
    // Assert authority, auction house instance and scopes are correct.
    assert_keys_equal(auctioneer_pda.auction_house, auction_house_instance.key())
        .map_err(|_e| AuctionHouseError::InvalidAuctioneer)?;
    Ok(())

}

pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> Result<()> {
    if sol_memcmp(key1.as_ref(), key2.as_ref(), PUBKEY_BYTES) != 0 {
        err!(AuctionHouseError::PublicKeyMismatch)
    } else {
        Ok(())
    }
}
