use anchor_lang::prelude::*;
use crate::account::*;
use crate::error::ErrorCode;

pub fn round_presale<'info>(
    presale_state: &Account<'info, PresaleState>,
    clock: &Sysvar<'info, Clock>,
) -> Result<()> {
    let presale_start_at = presale_state.presale_start_at;
    let presale_end_at = presale_state.presale_end_at;

    if presale_start_at > clock.unix_timestamp {
        return err!(ErrorCode::PresaleTooNew);
    }

    if presale_end_at < clock.unix_timestamp {
        return err!(ErrorCode::PresaleTooOld);
    }
    Ok(())
}