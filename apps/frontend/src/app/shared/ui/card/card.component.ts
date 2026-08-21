import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

export type CardTheme = 'parchment' | 'runic';
export type CardLayout = 'vertical' | 'horizontal' | 'auto';
export type CardMediaRatio = 'wide' | 'standard' | 'portrait';

@Component({
  selector: 'app-eivar-card',
  standalone: true,
  imports: [CommonModule],
  template: `
    <article
      class="eivar-card"
      [class.eivar-card--runic]="theme === 'runic'"
      [class.eivar-card--compact]="compact"
      [class.eivar-card--horizontal]="layout === 'horizontal'"
      [class.eivar-card--auto]="layout === 'auto'"
      [class.eivar-card--interactive]="interactive"
      [style.--card-media-ratio]="mediaRatioValue"
    >
      <span class="eivar-card__corner eivar-card__corner--top-left" aria-hidden="true"></span>
      <span class="eivar-card__corner eivar-card__corner--top-right" aria-hidden="true"></span>
      <span class="eivar-card__corner eivar-card__corner--bottom-left" aria-hidden="true"></span>
      <span class="eivar-card__corner eivar-card__corner--bottom-right" aria-hidden="true"></span>

      @if (image) {
        <div class="eivar-card__media">
          <img [src]="image" [alt]="imageAlt" />
          @if (badge) {
            <span class="eivar-card__badge">{{ badge }}</span>
          }
        </div>
      }

      <div class="eivar-card__content">
        @if (eyebrow) {
          <p class="eivar-card__eyebrow">{{ eyebrow }}</p>
        }
        @if (title) {
          <h3 class="eivar-card__title">{{ title }}</h3>
        }
        @if (description) {
          <p class="eivar-card__description">{{ description }}</p>
        }

        <ng-content select="[card-body]"></ng-content>

        <div class="eivar-card__footer">
          <ng-content select="[card-footer]"></ng-content>
        </div>
      </div>
    </article>
  `,
  styleUrls: ['./card.component.scss']
})
export class EivarCardComponent {
  @Input() image?: string;
  @Input() imageAlt = '';
  @Input() badge?: string;
  @Input() eyebrow?: string;
  @Input() title?: string;
  @Input() description?: string;
  @Input() theme: CardTheme = 'parchment';
  @Input() layout: CardLayout = 'vertical';
  @Input() mediaRatio: CardMediaRatio = 'standard';
  @Input() compact = false;
  @Input() interactive = false;

  get mediaRatioValue(): string {
    switch (this.mediaRatio) {
      case 'wide':
        return '16 / 7';
      case 'portrait':
        return '4 / 5';
      default:
        return '16 / 9';
    }
  }
}
