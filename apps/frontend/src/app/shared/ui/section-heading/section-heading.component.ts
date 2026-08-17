import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-section-heading',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="section-heading-container" [ngClass]="['align-' + align]">
      @if (badge) {
        <div class="category-badge">
          <span class="badge-rune">ᛟ</span>
          <span class="badge-text">{{ badge }}</span>
        </div>
      }
      <h2 class="main-title">
        <ng-content select="[title]"></ng-content>
        @if (title) {
          {{ title }}
        }
      </h2>
      @if (subtitle) {
        <p class="subtitle">{{ subtitle }}</p>
      }
    </div>
  `,
  styleUrls: ['./section-heading.component.scss']
})
export class SectionHeadingComponent {
  @Input() title?: string;
  @Input() subtitle?: string;
  @Input() badge?: string;
  @Input() align: 'left' | 'center' | 'right' = 'center';
}
