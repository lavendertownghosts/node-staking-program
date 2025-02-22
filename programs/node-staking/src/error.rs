use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Pool Authoricity is invalid")]
    InvalidPoolAuthority,
    #[msg("Nodes of Pool is overflowed")]
    AmountOverflow,
    #[msg("Presale is not started yet")]
    PresaleTooNew,
    #[msg("Presale is over")]
    PresaleTooOld,
    #[msg("Stakes amount is overflowed")]
    StakesAmountOverflow,
    #[msg("Pool doesn't have enough nodes")]
    LackNodes,
    #[msg("Max allocation is overflowed")]
    UserAmountOverflow,
    #[msg("Can not calcuate price for requested nodes")]
    UnableCalculatingNodesPrice,
    #[msg("Can not calcuate selling amount")]
    UnableCalculatingSellingTokens,
    #[msg("Insufficient balance for presale")]
    InsufficientBalanceForPresale,
    #[msg("Only owner can withdraw capital")]
    InvalidVaultAuthority,
    #[msg("Presale is not ended")]
    NotEndedPresale,
    #[msg("User token balance is not enough to buy nodes")]
    LackUserTokenBalance,
    #[msg("Can not calculate sum")]
    UnavailableCaculateSum,
    #[msg("Can not calculate sub")]
    UnavailableCaculateSub,
}
