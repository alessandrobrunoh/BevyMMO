//! [`Gold`] newtype and fee quoting.

use std::fmt;

/// One hundredth of a percent. `100` is 1%; [`BPS_DENOMINATOR`] is 100%.
pub const BPS_DENOMINATOR: u32 = 10_000;

/// Default account-wide fee applied on every fill until a subscription
/// (out of scope) writes `0` onto `account_economy`.
pub const DEFAULT_ACCOUNT_FEE_BPS: u16 = 100;

/// Character-scoped Gold. Not an item, not shared across an account's
/// characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Gold(u64);

impl Gold {
    pub const ZERO: Self = Self(0);

    pub const fn from_u64(amount: u64) -> Self {
        Self(amount)
    }

    pub const fn amount(self) -> u64 {
        self.0
    }

    /// Adds `amount`. Fails instead of wrapping past [`u64::MAX`].
    pub fn credit(self, amount: u64) -> Result<Self, GoldError> {
        self.0
            .checked_add(amount)
            .map(Self)
            .ok_or(GoldError::Overflow)
    }

    /// Subtracts `amount`. Fails instead of wrapping below zero.
    pub fn debit(self, amount: u64) -> Result<Self, GoldError> {
        self.0
            .checked_sub(amount)
            .map(Self)
            .ok_or(GoldError::Insufficient)
    }
}

impl fmt::Display for Gold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Why a Gold mutation or a fee quote could not complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoldError {
    Overflow,
    Insufficient,
    /// `price == 0` is not a legal listing or fill.
    ZeroPrice,
    /// Market + account fee would take the whole price (or more).
    FeeExceedsPrice,
}

impl fmt::Display for GoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => write!(f, "gold overflow"),
            Self::Insufficient => write!(f, "not enough gold"),
            Self::ZeroPrice => write!(f, "price must be greater than 0"),
            Self::FeeExceedsPrice => write!(f, "fee exceeds price"),
        }
    }
}

/// Breakdown of a fill: buyer pays the listed price, seller receives the
/// remainder after the summed fees, and the rest is burned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeQuote {
    pub market_bps: u16,
    pub account_bps: u16,
    pub total_bps: u32,
    pub fee_gold: u64,
    pub buyer_pays: Gold,
    pub seller_receives: Gold,
}

/// Quotes the fee on `price` as **market_bps + account_bps**, not compounded.
///
/// `fee_gold = price * total_bps / 10_000` using a `u128` intermediate so a
/// large price cannot wrap. Account bps of 0 (the future subscription case)
/// leaves only the market cut.
pub fn quote_fee(price: u64, market_bps: u16, account_bps: u16) -> Result<FeeQuote, GoldError> {
    if price == 0 {
        return Err(GoldError::ZeroPrice);
    }
    let total_bps = u32::from(market_bps) + u32::from(account_bps);
    let fee_gold = (u128::from(price) * u128::from(total_bps) / u128::from(BPS_DENOMINATOR)) as u64;
    if fee_gold >= price {
        return Err(GoldError::FeeExceedsPrice);
    }
    Ok(FeeQuote {
        market_bps,
        account_bps,
        total_bps,
        fee_gold,
        buyer_pays: Gold(price),
        seller_receives: Gold(price - fee_gold),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_and_debit_round_trip() {
        let gold = Gold::ZERO.credit(150).unwrap();
        assert_eq!(gold.amount(), 150);
        assert_eq!(gold.debit(50).unwrap().amount(), 100);
    }

    #[test]
    fn debit_fails_when_the_wallet_cannot_cover_the_amount() {
        let gold = Gold::from_u64(10);
        assert_eq!(gold.debit(11), Err(GoldError::Insufficient));
        assert_eq!(gold.debit(10).unwrap().amount(), 0);
        assert_eq!(Gold::ZERO.debit(1), Err(GoldError::Insufficient));
    }

    #[test]
    fn credit_fails_instead_of_wrapping() {
        assert_eq!(Gold::from_u64(u64::MAX).credit(1), Err(GoldError::Overflow));
        assert_eq!(
            Gold::from_u64(u64::MAX).credit(0).unwrap().amount(),
            u64::MAX
        );
    }

    #[test]
    fn quote_fee_sums_market_and_account_bps() {
        // 2% market + 1% account on 10_000 gold = 300 burned, seller 9_700.
        let quote = quote_fee(10_000, 200, 100).unwrap();
        assert_eq!(quote.total_bps, 300);
        assert_eq!(quote.fee_gold, 300);
        assert_eq!(quote.buyer_pays.amount(), 10_000);
        assert_eq!(quote.seller_receives.amount(), 9_700);
        assert_eq!(
            quote.buyer_pays.amount(),
            quote.seller_receives.amount() + quote.fee_gold
        );
    }

    #[test]
    fn zero_account_fee_leaves_only_the_market_cut() {
        let quote = quote_fee(10_000, 200, 0).unwrap();
        assert_eq!(quote.total_bps, 200);
        assert_eq!(quote.fee_gold, 200);
        assert_eq!(quote.seller_receives.amount(), 9_800);
    }

    #[test]
    fn both_fees_zero_is_a_full_transfer() {
        let quote = quote_fee(50, 0, 0).unwrap();
        assert_eq!(quote.fee_gold, 0);
        assert_eq!(quote.seller_receives.amount(), 50);
    }

    #[test]
    fn quote_fee_rejects_zero_price() {
        assert_eq!(quote_fee(0, 200, 100), Err(GoldError::ZeroPrice));
    }

    #[test]
    fn quote_fee_rejects_when_the_cut_takes_the_whole_price() {
        // 100% market takes the whole price.
        assert_eq!(quote_fee(100, 10_000, 0), Err(GoldError::FeeExceedsPrice));
        assert_eq!(quote_fee(1, 10_000, 100), Err(GoldError::FeeExceedsPrice));
    }

    #[test]
    fn quote_fee_uses_integer_division() {
        // 3% of 10 is 0.3 → 0 gold burned; seller still receives 10.
        let quote = quote_fee(10, 200, 100).unwrap();
        assert_eq!(quote.fee_gold, 0);
        assert_eq!(quote.seller_receives.amount(), 10);
    }
}
