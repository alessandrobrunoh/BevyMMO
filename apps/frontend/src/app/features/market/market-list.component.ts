import { Component, inject, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { MarketSummary } from '../../shared/models/market.model';
import { MarketService } from './market.service';

@Component({
  selector: 'app-market-list',
  standalone: true,
  imports: [CommonModule, RouterModule, PageHeaderComponent],
  templateUrl: './market-list.component.html',
  styleUrls: ['./market-list.component.scss']
})
export class MarketListComponent implements OnInit {
  private marketsApi = inject(MarketService);

  readonly markets = signal<MarketSummary[]>([]);
  readonly loading = signal(true);
  readonly error = signal<string | null>(null);

  async ngOnInit() {
    try {
      this.markets.set(await this.marketsApi.listMarkets());
    } catch (err) {
      this.error.set(err instanceof Error ? err.message : 'Could not load markets.');
    } finally {
      this.loading.set(false);
    }
  }
}
