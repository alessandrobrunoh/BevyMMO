import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { AuthService } from '../../core/services/auth.service';
import { ItemTicket, MarketSummary, SellOffer } from '../../shared/models/market.model';
import { MarketService } from './market.service';
import { getItemDetail, ItemDetailInfo } from './market-items.data';
import { MarketPriceChartComponent } from './components/market-chart/market-chart.component';
import { MarketCalculatorComponent } from './components/market-calculator/market-calculator.component';
import { DEFAULT_ACCOUNT_FEE_BPS, formatGold, quoteFee } from './market.utils';

@Component({
  selector: 'app-market-ticket',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    PageHeaderComponent,
    EivarButtonComponent,
    MarketPriceChartComponent,
    MarketCalculatorComponent
  ],
  templateUrl: './market-ticket.component.html',
  styleUrl: './market-ticket.component.scss'
})
export class MarketTicketComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private marketsApi = inject(MarketService);
  readonly auth = inject(AuthService);

  readonly marketId = signal('');
  readonly itemId = signal('');
  readonly market = signal<MarketSummary | null>(null);
  readonly ticket = signal<ItemTicket | null>(null);
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly unknownMarket = signal(false);

  readonly gold = this.marketsApi.gold;
  readonly selectedCharacterId = this.marketsApi.selectedCharacterId;
  readonly characters = computed(() => this.auth.profile()?.characters ?? []);

  readonly itemDetail = computed<ItemDetailInfo>(() => {
    return getItemDetail(this.itemId());
  });

  readonly isolatedSells = computed(() => {
    const ticket = this.ticket();
    if (!ticket || ticket.market_id !== this.marketId()) {
      return [] as SellOffer[];
    }
    return [...ticket.sell_orders].sort((a, b) => a.price_gold - b.price_gold);
  });

  readonly lowestAsk = computed(() => {
    const sells = this.isolatedSells();
    return sells.length > 0 ? sells[0].price_gold : 0;
  });

  readonly feeQuote = computed(() => {
    const market = this.market();
    const lowest = this.lowestAsk();
    if (!market || lowest === 0) return null;
    return quoteFee(lowest, market.fee_bps, DEFAULT_ACCOUNT_FEE_BPS);
  });

  readonly loginReturnUrl = computed(
    () => this.router.url.split('?')[0] || `/market/${this.marketId()}/${this.itemId()}`
  );

  readonly Math = Math;
  formatGold = formatGold;

  async ngOnInit() {
    const marketId = this.route.snapshot.paramMap.get('marketId') ?? '';
    const itemId = this.route.snapshot.paramMap.get('itemId') ?? '';
    this.marketId.set(marketId);
    this.itemId.set(itemId);
    await this.marketsApi.syncWallet();
    try {
      const markets = await this.marketsApi.listMarkets();
      const found = markets.find(m => m.id === marketId) ?? null;
      this.market.set(found);
      if (!found) {
        this.unknownMarket.set(true);
        return;
      }
      this.ticket.set(await this.marketsApi.getTicket(marketId, itemId));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Could not load ticket.';
      if (message.toLowerCase().includes('no market')) {
        this.unknownMarket.set(true);
      } else {
        this.error.set(message);
      }
    } finally {
      this.loading.set(false);
    }
  }

  async onCharacterChange(event: Event) {
    const id = (event.target as HTMLSelectElement).value;
    await this.marketsApi.syncWallet(id);
  }
}
