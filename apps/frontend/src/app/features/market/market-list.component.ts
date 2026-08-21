import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { MarketSummary, SellOffer } from '../../shared/models/market.model';
import { MarketService } from './market.service';
import { getItemDetail, ItemDetailInfo } from './market-items.data';
import { formatGold } from './market.utils';

export interface ListedItemEntry {
  item: ItemDetailInfo;
  marketId: string;
  marketName: string;
  marketFeeBps: number;
  offers: SellOffer[];
  lowestPrice: number;
  highestPrice: number;
  activeOffersCount: number;
}

@Component({
  selector: 'app-market-list',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    FormsModule,
    PageHeaderComponent,
    EivarButtonComponent
  ],
  templateUrl: './market-list.component.html',
  styleUrl: './market-list.component.scss'
})
export class MarketListComponent implements OnInit {
  private router = inject(Router);
  private marketsApi = inject(MarketService);

  readonly markets = signal<MarketSummary[]>([]);
  readonly allOffersMap = signal<Map<string, SellOffer[]>>(new Map());
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);

  readonly selectedMarketId = signal<string>('All');
  readonly selectedCategory = signal<string>('All');
  readonly searchQuery = signal<string>('');
  readonly sortBy = signal<'price_asc' | 'price_desc' | 'offers_desc' | 'name_asc'>('price_asc');

  readonly categories = ['All', 'Weapons', 'Armor', 'Accessories', 'Materials'];

  formatGold = formatGold;

  /**
   * Builds the complete list of items across all markets that have AT LEAST 1 active offer.
   */
  readonly activeListedItems = computed<ListedItemEntry[]>(() => {
    const marketList = this.markets();
    const offersMap = this.allOffersMap();
    const entries: ListedItemEntry[] = [];

    for (const market of marketList) {
      const marketOffers = offersMap.get(market.id) ?? [];
      if (marketOffers.length === 0) continue;

      // Group offers by item_id
      const itemGroupMap = new Map<string, SellOffer[]>();
      for (const offer of marketOffers) {
        if (!itemGroupMap.has(offer.item_id)) {
          itemGroupMap.set(offer.item_id, []);
        }
        itemGroupMap.get(offer.item_id)!.push(offer);
      }

      itemGroupMap.forEach((offers, itemId) => {
        if (offers.length > 0) {
          const sortedOffers = [...offers].sort((a, b) => a.price_gold - b.price_gold);
          const item = getItemDetail(itemId);

          entries.push({
            item,
            marketId: market.id,
            marketName: market.display_name,
            marketFeeBps: market.fee_bps,
            offers: sortedOffers,
            lowestPrice: sortedOffers[0].price_gold,
            highestPrice: sortedOffers[sortedOffers.length - 1].price_gold,
            activeOffersCount: sortedOffers.length
          });
        }
      });
    }

    return entries;
  });

  /**
   * Applies user filters: Market Select dropdown, Category, Search Query, and Sort.
   */
  readonly filteredItems = computed<ListedItemEntry[]>(() => {
    let list = this.activeListedItems();
    const marketFilter = this.selectedMarketId();
    const catFilter = this.selectedCategory();
    const query = this.searchQuery().trim().toLowerCase();
    const sort = this.sortBy();

    // Filter by Market
    if (marketFilter !== 'All') {
      list = list.filter(entry => entry.marketId === marketFilter);
    }

    // Filter by Category
    if (catFilter !== 'All') {
      list = list.filter(entry => entry.item.category === catFilter);
    }

    // Filter by Search text
    if (query) {
      list = list.filter(
        entry =>
          entry.item.name.toLowerCase().includes(query) ||
          entry.item.id.toLowerCase().includes(query) ||
          entry.item.subType.toLowerCase().includes(query) ||
          entry.marketName.toLowerCase().includes(query)
      );
    }

    // Sort
    return [...list].sort((a, b) => {
      if (sort === 'price_asc') return a.lowestPrice - b.lowestPrice;
      if (sort === 'price_desc') return b.lowestPrice - a.lowestPrice;
      if (sort === 'offers_desc') return b.activeOffersCount - a.activeOffersCount;
      if (sort === 'name_asc') return a.item.name.localeCompare(b.item.name);
      return 0;
    });
  });

  async ngOnInit() {
    try {
      const markets = await this.marketsApi.listMarkets();
      this.markets.set(markets);

      // Load offers from all markets concurrently
      const offersMap = new Map<string, SellOffer[]>();
      await Promise.all(
        markets.map(async market => {
          try {
            const offers = await this.marketsApi.listOffers(market.id);
            offersMap.set(market.id, offers);
          } catch {
            offersMap.set(market.id, []);
          }
        })
      );
      this.allOffersMap.set(offersMap);
    } catch (err) {
      this.error.set(err instanceof Error ? err.message : 'Could not load market data.');
    } finally {
      this.loading.set(false);
    }
  }

  openTicket(entry: ListedItemEntry) {
    this.router.navigate(['/market', entry.marketId, entry.item.id]);
  }
}
