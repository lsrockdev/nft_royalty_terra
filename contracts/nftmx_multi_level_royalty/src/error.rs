use cosmwasm_std::StdError;
use thiserror::Error;
use cosmwasm_std::Uint128;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Token Id are different")]
    TokenIdDifferent {},

    #[error("NFT address are not matched")]
    NotMatchNFT {},

    #[error("Price is not matched")]
    NotMatchPrice {},

    #[error("Protection Rate is not valid")]
    InvalidProtectionRate {},

    #[error("Protection time is different")]
    InvalidProtectionTime {},

    #[error("typeorder should be AuctionType")]
    NoAuctionType{},

    #[error("typeorder should be FixedPay")]
    NoFixedPay{},

    #[error("This order should be done by buyer")]
    InvalidBuyer{},

    #[error("Sell order is still alive")]
    SellOrderAlive{},

    #[error("Insufficient offer time")]
    InsufficientOfferTime{},

    #[error("Protection time expired")]
    ExpiredProtectionTime{},

    #[error("Price is not matched")]
    PriceNotMatch{},

    #[error("Invalid coin amount")]
    InvalidZeroAmount{},

    #[error("Funding is ended")]
    FundingIsEnded{},
    
    #[error("Minting cannot exceed the cap")]
    CannotExceedCap{},

    #[error("Funding is ended.")]
    FundingEnded{},
    
    #[error("Balance Insufficient")]
    BalanceInsufficient{},

    #[error("Pool Balance is Insufficient")]
    PoolBalanceInsufficient{},

}
