import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule, ActivatedRoute } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { AuthService } from '../../core/services/auth.service';
import { MarketSummary, SellOffer } from '../../shared/models/market.model';
import { MarketService } from './market.service';
import { displayItemName, filterOffersByMarketId, formatGold } from './market.utils';

export interface GroupedItemSummary {
  itemId: string;
  name: string;
  offers: SellOffer[];
  lowestPrice: number;
  highestPrice: number;
  totalListings: number;
}

@Component({
  selector: 'app-market-browse',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    FormsModule,
    PageHeaderComponent,
    EivarButtonComponent
  ],
  templateUrl: './market-browse.component.html',
  styleUrl: './market-browse.component.scss'
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
  readonly sortBy = signal<'price_asc' | 'price_desc' | 'name_asc'>('price_asc');
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly unknownMarket = signal(false);

  readonly gold = this.marketsApi.gold;
  readonly selectedCharacterId = this.marketsApi.selectedCharacterId;
  readonly characters = computed(() => this.auth.profile()?.characters ?? []);

  readonly groupedItems = computed<GroupedItemSummary[]>(() => {
    const rawOffers = this.offers();
    const marketId = this.marketId();
    const scoped = filterOffersByMarketId(
      rawOffers.map(offer => ({
        ...offer,
        market_id: (offer as SellOffer & { market_id?: string }).market_id ?? marketId
      })),
      marketId
    );

    const map = new Map<string, SellOffer[]>();
    for (const offer of scoped) {
      if (!map.has(offer.item_id)) {
        map.set(offer.item_id, []);
      }
      map.get(offer.item_id)!.push(offer);
    }

    const hall = this.market();
    if (hall && hall.allowed_item_ids) {
      for (const allowedId of hall.allowed_item_ids) {
        if (!map.has(allowedId)) {
          map.set(allowedId, []);
        }
      }
    }

    const results: GroupedItemSummary[] = [];
    map.forEach((offerList, itemId) => {
      const sorted = [...offerList].sort((a, b) => a.price_gold - b.price_gold);
      results.push({
        itemId,
        name: displayItemName(itemId),
        offers: sorted,
        lowestPrice: sorted.length > 0 ? sorted[0].price_gold : 0,
        highestPrice: sorted.length > 0 ? sorted[sorted.length - 1].price_gold : 0,
        totalListings: sorted.length
      });
    });

    return results;
  });

  readonly visibleItems = computed<GroupedItemSummary[]>(() => {
    let list = this.groupedItems();
    const needle = this.search().trim().toLowerCase();
    if (needle) {
      list = list.filter(
        item => item.name.toLowerCase().includes(needle) || item.itemId.toLowerCase().includes(needle)
      );
    }

    const sort = this.sortBy();
    return [...list].sort((a, b) => {
      if (sort === 'price_asc') return a.lowestPrice - b.lowestPrice;
      if (sort === 'price_desc') return b.lowestPrice - a.lowestPrice;
      if (sort === 'name_asc') return a.name.localeCompare(b.name);
      return 0;
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

  clearSearch() {
    this.search.set('');
  }

  async onCharacterChange(event: Event) {
    const id = (event.target as HTMLSelectElement).value;
    await this.marketsApi.syncWallet(id);
  }

  openTicket(itemId: string) {
    this.router.navigate(['/market', this.marketId(), itemId]);
  }
}
