import { Component, computed, input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { SellOffer } from '../../../../shared/models/market.model';
import { formatGold } from '../../market.utils';

interface PriceBucket {
  price: number;
  count: number;
  x: number;
  y: number;
  height: number;
}

@Component({
  selector: 'app-market-chart',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="market-chart-card">
      <div class="chart-card-header">
        <div class="header-left">
          <span class="chart-rune">ᛟ</span>
          <h3 class="chart-title">Offer Depth & Price Curve</h3>
        </div>

        @if (offers().length > 0) {
          <div class="chart-stats-pills">
            <span class="stat-pill">Lowest: <strong>{{ formatGold(lowestPrice()) }}</strong></span>
            <span class="stat-pill">Avg: <strong>{{ formatGold(averagePrice()) }}</strong></span>
            <span class="stat-pill">Highest: <strong>{{ formatGold(highestPrice()) }}</strong></span>
          </div>
        }
      </div>

      @if (offers().length === 0) {
        <div class="chart-empty">
          <span class="empty-rune">ᛟ</span>
          <p>No active listings in the book to chart.</p>
        </div>
      } @else {
        <div class="chart-svg-wrap">
          <svg [attr.viewBox]="'0 0 ' + width + ' ' + height" class="chart-svg" preserveAspectRatio="none">
            <defs>
              <linearGradient id="parchmentAreaGrad" x1="0%" y1="0%" x2="0%" y2="100%">
                <stop offset="0%" stop-color="#1c91d0" stop-opacity="0.3" />
                <stop offset="100%" stop-color="#1c91d0" stop-opacity="0.02" />
              </linearGradient>
            </defs>

            <!-- Horizontal grid lines -->
            @for (level of gridLines(); track level.y) {
              <line
                [attr.x1]="paddingLeft"
                [attr.y1]="level.y"
                [attr.x2]="width - paddingRight"
                [attr.y2]="level.y"
                stroke="rgba(117, 107, 90, 0.2)"
                stroke-width="1"
                stroke-dasharray="3 3"
              />
              <text
                [attr.x]="width - paddingRight + 6"
                [attr.y]="level.y + 4"
                class="axis-label"
              >
                {{ level.price }}g
              </text>
            }

            <!-- Area & Line Curve of Asks -->
            @if (areaPath()) {
              <path [attr.d]="areaPath()" fill="url(#parchmentAreaGrad)" />
            }
            @if (linePath()) {
              <path
                [attr.d]="linePath()"
                fill="none"
                stroke="#147cc1"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            }

            <!-- Order Points -->
            @for (pt of points(); track pt.id) {
              <circle
                [attr.cx]="pt.x"
                [attr.cy]="pt.y"
                r="4.5"
                fill="#147cc1"
                stroke="#ffffff"
                stroke-width="1.5"
              />
            }

            <!-- X Axis Baseline -->
            <line
              [attr.x1]="paddingLeft"
              [attr.y1]="height - paddingBottom"
              [attr.x2]="width - paddingRight"
              [attr.y2]="height - paddingBottom"
              stroke="rgba(117, 107, 90, 0.4)"
              stroke-width="1.5"
            />

            <!-- X Axis Labels -->
            <text [attr.x]="paddingLeft" [attr.y]="height - 6" text-anchor="start" class="axis-label">
              Order #1 (Best Ask: {{ lowestPrice() }}g)
            </text>
            <text [attr.x]="width - paddingRight" [attr.y]="height - 6" text-anchor="end" class="axis-label">
              Order #{{ offers().length }} ({{ highestPrice() }}g)
            </text>
          </svg>
        </div>
      }
    </div>
  `,
  styleUrl: './market-chart.component.scss'
})
export class MarketPriceChartComponent {
  readonly offers = input<SellOffer[]>([]);

  readonly width = 600;
  readonly height = 200;
  readonly paddingTop = 20;
  readonly paddingBottom = 28;
  readonly paddingLeft = 20;
  readonly paddingRight = 60;

  formatGold = formatGold;

  readonly sortedOffers = computed(() => {
    return [...this.offers()].sort((a, b) => a.price_gold - b.price_gold);
  });

  readonly lowestPrice = computed(() => {
    const list = this.sortedOffers();
    return list.length > 0 ? list[0].price_gold : 0;
  });

  readonly highestPrice = computed(() => {
    const list = this.sortedOffers();
    return list.length > 0 ? list[list.length - 1].price_gold : 0;
  });

  readonly averagePrice = computed(() => {
    const list = this.sortedOffers();
    if (list.length === 0) return 0;
    const total = list.reduce((sum, o) => sum + o.price_gold, 0);
    return Math.round(total / list.length);
  });

  readonly minBound = computed(() => {
    const low = this.lowestPrice();
    return Math.max(0, Math.floor(low * 0.9));
  });

  readonly maxBound = computed(() => {
    const high = this.highestPrice();
    return Math.ceil(high * 1.1) || 100;
  });

  readonly priceRange = computed(() => {
    const range = this.maxBound() - this.minBound();
    return range <= 0 ? 1 : range;
  });

  readonly gridLines = computed(() => {
    const min = this.minBound();
    const range = this.priceRange();
    const topY = this.paddingTop;
    const bottomY = this.height - this.paddingBottom;
    const heightSpan = bottomY - topY;

    const levels = [];
    for (let i = 0; i <= 3; i++) {
      const price = Math.round(min + (i / 3) * range);
      const y = bottomY - (i / 3) * heightSpan;
      levels.push({ price, y });
    }
    return levels;
  });

  readonly points = computed(() => {
    const list = this.sortedOffers();
    if (list.length === 0) return [];
    const availableWidth = this.width - this.paddingLeft - this.paddingRight;
    const topY = this.paddingTop;
    const bottomY = this.height - this.paddingBottom;
    const heightSpan = bottomY - topY;
    const min = this.minBound();
    const range = this.priceRange();

    return list.map((offer, idx) => {
      const x =
        list.length === 1
          ? this.paddingLeft + availableWidth / 2
          : this.paddingLeft + (idx / (list.length - 1)) * availableWidth;
      const y = bottomY - ((offer.price_gold - min) / range) * heightSpan;
      return {
        id: offer.id,
        price: offer.price_gold,
        x,
        y
      };
    });
  });

  readonly linePath = computed(() => {
    const pts = this.points();
    if (pts.length === 0) return '';
    return pts.reduce(
      (acc, curr, i) => `${acc} ${i === 0 ? 'M' : 'L'} ${curr.x.toFixed(1)} ${curr.y.toFixed(1)}`,
      ''
    );
  });

  readonly areaPath = computed(() => {
    const pts = this.points();
    if (pts.length === 0) return '';
    const bottomY = this.height - this.paddingBottom;
    const first = pts[0];
    const last = pts[pts.length - 1];
    return `M ${first.x.toFixed(1)} ${bottomY} L ${this.linePath().substring(2)} L ${last.x.toFixed(1)} ${bottomY} Z`;
  });
}
