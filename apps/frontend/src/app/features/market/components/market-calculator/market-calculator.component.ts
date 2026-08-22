import { Component, computed, input, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { EivarButtonComponent } from '../../../../shared/ui/button/button.component';
import { formatGold } from '../../market.utils';

@Component({
  selector: 'app-market-calculator',
  standalone: true,
  imports: [CommonModule, FormsModule, EivarButtonComponent],
  template: `
    <div class="calc-card">
      <div class="calc-head">
        <span class="calc-rune">⚖</span>
        <h3 class="calc-title">Listing & Fee Calculator</h3>
      </div>

      <div class="calc-inputs-row">
        <div class="input-box">
          <label>Listing Price (Gold / Unit)</label>
          <div class="input-control">
            <span class="currency-icon">✦</span>
            <input
              type="number"
              [ngModel]="targetPrice()"
              (ngModelChange)="targetPrice.set($event)"
              min="1"
            />
          </div>
        </div>

        <div class="input-box">
          <label>Quantity</label>
          <div class="input-control">
            <input
              type="number"
              [ngModel]="quantity()"
              (ngModelChange)="setQuantity($event)"
              min="1"
            />
          </div>
          <div class="qty-presets">
            <app-eivar-button variant="tag" size="sm" [active]="quantity() === 1" [toggle]="true" (onClick)="setQuantity(1)">1x</app-eivar-button>
            <app-eivar-button variant="tag" size="sm" [active]="quantity() === 5" [toggle]="true" (onClick)="setQuantity(5)">5x</app-eivar-button>
            <app-eivar-button variant="tag" size="sm" [active]="quantity() === 10" [toggle]="true" (onClick)="setQuantity(10)">10x</app-eivar-button>
            <app-eivar-button variant="tag" size="sm" [active]="quantity() === 25" [toggle]="true" (onClick)="setQuantity(25)">25x</app-eivar-button>
          </div>
        </div>
      </div>

      <div class="calc-summary-panel">
        <div class="summary-line">
          <span>Gross Value ({{ quantity() }} @ {{ targetPrice() }}g)</span>
          <span class="line-val">{{ formatGold(grossGold()) }}</span>
        </div>
        <div class="summary-line fee-line">
          <span>Market Fee ({{ marketFeeBps() }} bps) + Account ({{ accountFeeBps() }} bps)</span>
          <span class="line-val fee-highlight">-{{ formatGold(totalFeeAmount()) }} ({{ totalBps() / 100 }}%)</span>
        </div>
        <div class="summary-line">
          <span>Fee per Unit</span>
          <span class="line-val">{{ formatGold(feePerUnit()) }} / unit</span>
        </div>
        <div class="summary-line net-total">
          <span>Net Proceeds to Seller</span>
          <span class="line-val net-val">{{ formatGold(netProceeds()) }}</span>
        </div>
      </div>
    </div>
  `,
  styleUrl: './market-calculator.component.scss'
})
export class MarketCalculatorComponent {
  readonly currentPrice = input<number>(100);
  readonly suggestedQuantity = input<number>(1);
  readonly marketFeeBps = input<number>(200);
  readonly accountFeeBps = input<number>(100);

  readonly targetPrice = signal<number>(100);
  readonly quantity = signal<number>(1);

  formatGold = formatGold;

  constructor() {
    setTimeout(() => {
      this.targetPrice.set(Math.max(1, this.currentPrice()));
      this.setQuantity(this.suggestedQuantity());
    }, 0);
  }

  setQuantity(value: number) {
    const amount = Number.isFinite(value) ? Math.floor(value) : 1;
    this.quantity.set(Math.max(1, amount));
  }

  readonly totalBps = computed(() => this.marketFeeBps() + this.accountFeeBps());

  readonly grossGold = computed(() => this.targetPrice() * this.quantity());

  readonly totalFeeAmount = computed(() => {
    const gross = this.grossGold();
    return Math.floor((gross * this.totalBps()) / 10000);
  });

  readonly feePerUnit = computed(() => {
    return Math.floor((this.targetPrice() * this.totalBps()) / 10000);
  });

  readonly netProceeds = computed(() => this.grossGold() - this.totalFeeAmount());
}
