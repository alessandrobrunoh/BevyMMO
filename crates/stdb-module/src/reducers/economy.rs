//! Character Gold: wallets, grants, and (later) market fills.
//!
//! Gold lives on [`CharacterWallet`], keyed by `character_id`. The arithmetic
//! is [`bevymmo_domain::economy::Gold`] so a fill and a GM grant cannot
//! disagree about overflow.

use spacetimedb::{reducer, ReducerContext, Table};

use bevymmo_domain::economy::{Gold, GoldError, DEFAULT_ACCOUNT_FEE_BPS};

use crate::tables::{
    account_economy, character_wallet, player, AccountEconomy, CharacterWallet,
};
use crate::{normalize_name, world};

/// Inserts a zero-Gold wallet if this character does not have one yet.
///
/// `join` always inserts; this exists so a grant (or a later market fill)
/// against a character created before the table existed does not fail.
pub fn ensure_wallet(ctx: &ReducerContext, character_id: spacetimedb::Uuid) -> CharacterWallet {
    if let Some(row) = ctx.db.character_wallet().character_id().find(character_id) {
        return row;
    }
    ctx.db.character_wallet().insert(CharacterWallet {
        character_id,
        gold: 0,
    })
}

/// Inserts the default account fee if this account has no economy row yet.
pub fn ensure_account_economy(ctx: &ReducerContext, account_id: u64) -> AccountEconomy {
    if let Some(row) = ctx.db.account_economy().account_id().find(account_id) {
        return row;
    }
    ctx.db.account_economy().insert(AccountEconomy {
        account_id,
        fee_bps: DEFAULT_ACCOUNT_FEE_BPS,
    })
}

/// Credits `amount` onto `character_id`. Used by GM grant and, later, market
/// fills. Amount `0` is rejected so a no-op cannot be mistaken for a grant.
pub fn credit_gold(
    ctx: &ReducerContext,
    character_id: spacetimedb::Uuid,
    amount: u64,
) -> Result<CharacterWallet, String> {
    let amount = grant_amount(amount)?;
    let wallet = ensure_wallet(ctx, character_id);
    let gold = Gold::from_u64(wallet.gold)
        .credit(amount)
        .map_err(gold_err)?;
    let wallet = CharacterWallet {
        character_id,
        gold: gold.amount(),
    };
    ctx.db
        .character_wallet()
        .character_id()
        .update(CharacterWallet {
            character_id: wallet.character_id,
            gold: wallet.gold,
        });
    Ok(wallet)
}

/// Debits `amount` from `character_id`. Fails with "not enough gold" rather
/// than wrapping. Used by market fills.
pub fn debit_gold(
    ctx: &ReducerContext,
    character_id: spacetimedb::Uuid,
    amount: u64,
) -> Result<CharacterWallet, String> {
    let wallet = ensure_wallet(ctx, character_id);
    let gold = Gold::from_u64(wallet.gold)
        .debit(amount)
        .map_err(gold_err)?;
    let wallet = CharacterWallet {
        character_id,
        gold: gold.amount(),
    };
    ctx.db
        .character_wallet()
        .character_id()
        .update(CharacterWallet {
            character_id: wallet.character_id,
            gold: wallet.gold,
        });
    Ok(wallet)
}

fn grant_amount(amount: u64) -> Result<u64, String> {
    if amount == 0 {
        return Err("amount must be greater than 0".to_string());
    }
    Ok(amount)
}

fn gold_err(err: GoldError) -> String {
    err.to_string()
}

/// Credits Gold onto the character with this display name.
///
/// GM-only. Looks up the character by normalized name so the caller types
/// what they see in the world, not a UUID.
#[reducer]
pub fn gm_grant_gold(
    ctx: &ReducerContext,
    display_name: String,
    amount: u64,
) -> Result<(), String> {
    world::require_gm(ctx)?;
    let amount = grant_amount(amount)?;
    let key = normalize_name(&display_name);
    if key.is_empty() {
        return Err("character name cannot be empty".to_string());
    }
    let character = ctx
        .db
        .player()
        .normalized_name()
        .find(&key)
        .ok_or_else(|| format!("no character named {display_name:?}"))?;
    credit_gold(ctx, character.character_id, amount)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_amount_rejects_zero() {
        assert!(grant_amount(0).is_err());
        assert_eq!(grant_amount(1).unwrap(), 1);
    }

    #[test]
    fn credit_overflow_is_an_error_string() {
        let err = Gold::from_u64(u64::MAX)
            .credit(1)
            .map_err(gold_err)
            .unwrap_err();
        assert_eq!(err, "gold overflow");
    }
}
