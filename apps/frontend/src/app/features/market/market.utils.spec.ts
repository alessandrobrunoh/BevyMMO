import {
  filterOffersByMarketId,
  quoteFee,
  displayItemName,
  offerQuantity,
  unitPriceGold,
  formatOfferPrice
} from './market.utils';

describe('market isolation', () => {
  const mixed = [
    { id: 1, market_id: 'market_1', item_id: 'sword', quantity: 1, price_gold: 100, seller_character_id: 'a' },
    { id: 2, market_id: 'market_2', item_id: 'simple_helm', quantity: 1, price_gold: 40, seller_character_id: 'b' },
    { id: 3, market_id: 'market_1', item_id: 'bow', quantity: 1, price_gold: 80, seller_character_id: 'c' }
  ];

  it('never lists Market 2 offers on the Market 1 page', () => {
    const one = filterOffersByMarketId(mixed, 'market_1');
    expect(one.map(o => o.item_id)).toEqual(['sword', 'bow']);
    expect(one.every(o => o.market_id === 'market_1')).toBe(true);
  });

  it('never lists Market 1 offers on the Market 2 page', () => {
    const two = filterOffersByMarketId(mixed, 'market_2');
    expect(two).toHaveLength(1);
    expect(two[0].item_id).toBe('simple_helm');
  });

  it('empty state when a market has no orders', () => {
    expect(filterOffersByMarketId(mixed, 'market_9')).toEqual([]);
  });
});

describe('market fees', () => {
  it('quotes floor(price * (market + account) / 10000)', () => {
    const quote = quoteFee(10_000, 200, 100);
    expect(quote.fee).toBe(300);
    expect(quote.youPay).toBe(10_000);
    expect(quote.sellerReceives).toBe(9_700);
  });
});

describe('item labels', () => {
  it('title-cases catalog ids', () => {
    expect(displayItemName('simple_helm')).toBe('Simple Helm');
  });
});

describe('listing quantity', () => {
  it('treats missing or zero quantity as one unit', () => {
    expect(offerQuantity({})).toBe(1);
    expect(offerQuantity({ quantity: 0 })).toBe(1);
    expect(offerQuantity({ quantity: 50 })).toBe(50);
  });

  it('derives unit price from the listed total', () => {
    expect(unitPriceGold(50, 10)).toBe(5);
    expect(unitPriceGold(100, 1)).toBe(100);
    expect(unitPriceGold(7, 3)).toBe(2);
  });

  it('formats stacked offers as quantity times unit gold', () => {
    expect(formatOfferPrice(100, 1)).toBe('100 Gold');
    expect(formatOfferPrice(50, 10)).toBe('10 × 5g');
  });
});
