import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-section-heading',
  standalone: true,
  imports: [CommonModule],
  template: `
    <header class="section-heading" [ngClass]="['align-' + align, theme]">
      <div class="rune-divider" [ngClass]="theme === 'dark' ? 'gold' : ''">
        <span>◇</span>
      </div>

      @if (badge) {
        <span class="eyebrow">{{ badge }}</span>
      }

      <h2>
        @if (title) {
          {{ title }}
        }
        <ng-content select="[title]"></ng-content>
      </h2>

      @if (subtitle) {
        <p>{{ subtitle }}</p>
      }
    </header>
  `,
  styleUrls: ['./section-heading.component.scss']
})
export class SectionHeadingComponent {
  @Input() title?: string;
  @Input() subtitle?: string;
  @Input() badge?: string;
  @Input() align: 'left' | 'center' | 'right' = 'center';
  @Input() theme: 'light' | 'dark' = 'light';
}
