import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { AuthService } from '../../core/services/auth.service';
import { MarketSummary, SellOffer } from '../../shared/models/market.model';
import { MarketService } from './market.service';
import { displayItemName, filterOffersByMarketId, formatGold } from './market.utils';

@Component({
  selector: 'app-market-browse',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent, EivarButtonComponent],
  templateUrl: './market-browse.component.html',
  styleUrls: ['./market-browse.component.scss']
})
export class MarketBrowseComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private marketsApi = inject(MarketService);
  readonly auth = inject(AuthService);

  readonly marketId = signal('');
  readonly market = signal<MarketSummary | null>(null);
  readonly offers = signal<SellOffer[]>([]);
  readonly search = signal('');
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly unknownMarket = signal(false);

  readonly gold = this.marketsApi.gold;
  readonly selectedCharacterId = this.marketsApi.selectedCharacterId;
  readonly characters = computed(() => this.auth.profile()?.characters ?? []);

  readonly visibleOffers = computed(() => {
    const needle = this.search().trim().toLowerCase();
    const scoped = filterOffersByMarketId(
      this.offers().map(offer => ({
        ...offer,
        market_id: (offer as SellOffer & { market_id?: string }).market_id ?? this.marketId()
      })),
      this.marketId()
    );
    if (!needle) return scoped;
    return scoped.filter(offer => {
      const name = displayItemName(offer.item_id).toLowerCase();
      return name.includes(needle) || offer.item_id.toLowerCase().includes(needle);
    });
  });

  readonly loginReturnUrl = computed(
    () => this.router.url.split('?')[0] || `/market/${this.marketId()}`
  );

  displayItemName = displayItemName;
  formatGold = formatGold;

  async ngOnInit() {
    const marketId = this.route.snapshot.paramMap.get('marketId') ?? '';
    this.marketId.set(marketId);
    await this.marketsApi.syncWallet();
    try {
      const markets = await this.marketsApi.listMarkets();
      const found = markets.find(m => m.id === marketId) ?? null;
      this.market.set(found);
      if (!found) {
        this.unknownMarket.set(true);
        return;
      }
      this.offers.set(await this.marketsApi.listOffers(marketId));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Could not load offers.';
      if (message.toLowerCase().includes('no market')) {
        this.unknownMarket.set(true);
      } else {
        this.error.set(message);
      }
    } finally {
      this.loading.set(false);
    }
  }

  onSearch(event: Event) {
    this.search.set((event.target as HTMLInputElement).value);
  }

  async onCharacterChange(event: Event) {
    const id = (event.target as HTMLSelectElement).value;
    await this.marketsApi.syncWallet(id);
  }

  openTicket(itemId: string) {
    this.router.navigate(['/market', this.marketId(), itemId]);
  }
}
