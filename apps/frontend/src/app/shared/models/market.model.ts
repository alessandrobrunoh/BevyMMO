export interface MarketSummary {
  id: string;
  display_name: string;
  fee_bps: number;
  allowed_item_ids: string[];
}

export interface SellOffer {
  id: number;
  item_id: string;
  price_gold: number;
  seller_character_id: string;
}

export interface BuyOffer {
  id: number;
  item_id: string;
  price_gold: number;
  buyer_character_id: string;
}

export interface ItemTicket {
  market_id: string;
  item_id: string;
  sell_orders: SellOffer[];
  buy_orders: BuyOffer[];
}

export interface WalletResponse {
  gold: number;
}

export interface FeeQuote {
  marketBps: number;
  accountBps: number;
  fee: number;
  youPay: number;
  sellerReceives: number;
}
