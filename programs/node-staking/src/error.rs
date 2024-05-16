use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Pool Authoricity is invalid")]
    InvalidPoolAuthority,
    #[msg("Nodes of Pool is overflowed")]
    AmountOverflow,
}
