import { HttpErrorResponse } from '@angular/common/http';
import { filterOffersByMarketId } from './market.utils';
import { SellOffer } from '../../shared/models/market.model';
import { describeGatewayError } from './market.service';

describe('MarketService isolation contract', () => {
  it('drops foreign-market rows if a mixed payload is ever returned', () => {
    const payload: Array<SellOffer & { market_id: string }> = [
      { id: 1, item_id: 'sword', price_gold: 50, seller_character_id: 'x', market_id: 'market_1' },
      { id: 2, item_id: 'simple_cuirass', price_gold: 70, seller_character_id: 'y', market_id: 'market_2' }
    ];
    const forHall = (marketId: string) =>
      filterOffersByMarketId(payload, marketId).map(row => row.item_id);

    expect(forHall('market_1')).toEqual(['sword']);
    expect(forHall('market_2')).toEqual(['simple_cuirass']);
  });
});

describe('describeGatewayError', () => {
  it('explains an HTML 200 from the Angular SPA fallback', () => {
    const err = new HttpErrorResponse({
      status: 200,
      statusText: 'OK',
      url: '/v1/public/markets',
      error: { error: new SyntaxError('Unexpected token <') }
    });
    expect(describeGatewayError(err, '/public/markets')).toContain('gateway is not reachable');
  });
});
