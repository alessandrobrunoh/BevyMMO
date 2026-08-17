import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { WikiInfoBox } from '../../models/wiki.model';

@Component({
  selector: 'app-wiki-infobox',
  standalone: true,
  imports: [CommonModule],
  template: `
    @if (data) {
      <aside class="wiki-infobox chamfer-box">
        <header class="infobox-header">
          <span class="rune-tag">ᛟ</span>
          <h4 class="infobox-title">{{ data.title }}</h4>
          <span class="infobox-type">{{ data.type }}</span>
        </header>

        @if (data.image) {
          <div class="infobox-media">
            <img [src]="data.image" [alt]="data.title" class="infobox-img" />
          </div>
        }

        <div class="infobox-stats">
          @for (stat of data.stats; track stat.label) {
            <div class="stat-row" [class.highlight]="stat.highlight">
              <span class="stat-label">{{ stat.label }}</span>
              <span class="stat-value">{{ stat.value }}</span>
            </div>
          }
        </div>

        @if (data.rarity) {
          <footer class="infobox-footer">
            <span class="rarity-badge" [ngClass]="data.rarity.toLowerCase()">
              {{ data.rarity }} Tier
            </span>
          </footer>
        }
      </aside>
    }
  `,
  styleUrls: ['./wiki-infobox.component.scss']
})
export class WikiInfoBoxComponent {
  @Input() data?: WikiInfoBox;
}
