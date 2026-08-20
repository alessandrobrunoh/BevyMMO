//! Character Gold and market fee quotes.
//!
//! Gold is a character-scoped currency, not an inventory item. Arithmetic
//! never wraps: overflow and insufficient funds are errors so a reducer can
//! abort the whole transaction.
//!
//! Fees are two addends in basis points (100 = 1%): the market's cut plus
//! the account's cut. A future subscription sets the account addend to zero
//! without touching the market addend.

pub mod gold;

pub use gold::{quote_fee, FeeQuote, Gold, GoldError, BPS_DENOMINATOR, DEFAULT_ACCOUNT_FEE_BPS};
