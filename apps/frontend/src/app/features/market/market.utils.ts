import { FeeQuote } from '../../shared/models/market.model';

/** Matches `bevymmo_gameplay::economy::DEFAULT_ACCOUNT_FEE_BPS`. */
export const DEFAULT_ACCOUNT_FEE_BPS = 100;

/** Matches `bevymmo_gameplay::economy::BPS_DENOMINATOR`. */
export const BPS_DENOMINATOR = 10_000;

/**
 * Isolation: Market 1's offers must never appear on Market 2.
 * The gateway already filters; the UI reapplies this so a mixed payload
 * cannot leak across pages. Records used in tests carry `market_id`.
 */
export function filterOffersByMarketId<T extends { market_id: string }>(
  offers: T[],
  marketId: string
): T[] {
  return offers.filter(offer => offer.market_id === marketId);
}

export function displayItemName(itemId: string): string {
  return itemId
    .split('_')
    .filter(part => part.length > 0)
    .map(part => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}

/** `fee = floor(price * (market_bps + account_bps) / 10000)`. */
export function quoteFee(
  price: number,
  marketBps: number,
  accountBps = DEFAULT_ACCOUNT_FEE_BPS
): FeeQuote {
  const fee = Math.floor((price * (marketBps + accountBps)) / BPS_DENOMINATOR);
  return {
    marketBps,
    accountBps,
    fee,
    youPay: price,
    sellerReceives: price - fee
  };
}

export function formatGold(amount: number): string {
  return `${amount.toLocaleString('en-US')} Gold`;
}

/** Listed pile size. Missing or zero (legacy rows) reads as 1. */
export function offerQuantity(offer: { quantity?: number }): number {
  const quantity = offer.quantity ?? 1;
  return quantity > 0 ? quantity : 1;
}

/** Integer gold per unit for a listed pile. */
export function unitPriceGold(priceGold: number, quantity: number): number {
  return Math.floor(priceGold / offerQuantity({ quantity }));
}

/** `10 × 5g` for stacks, otherwise the total in Gold. */
export function formatOfferPrice(priceGold: number, quantity: number): string {
  const qty = offerQuantity({ quantity });
  if (qty === 1) {
    return formatGold(priceGold);
  }
  const unit = unitPriceGold(priceGold, qty);
  return `${qty.toLocaleString('en-US')} × ${unit.toLocaleString('en-US')}g`;
}
