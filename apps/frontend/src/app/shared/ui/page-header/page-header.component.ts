import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

@Component({
  selector: 'app-page-header',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <header class="page-header" [style.background-image]="bgImage ? 'url(' + bgImage + ')' : null">
      <div class="header-overlay"></div>
      <div class="container header-content">
        @if (breadcrumbs && breadcrumbs.length > 0) {
          <nav class="breadcrumbs" aria-label="Breadcrumb">
            <a routerLink="/" class="crumb-link">Home</a>
            @for (crumb of breadcrumbs; track crumb.label) {
              <span class="crumb-sep">/</span>
              @if (crumb.route) {
                <a [routerLink]="crumb.route" class="crumb-link">{{ crumb.label }}</a>
              } @else {
                <span class="crumb-current">{{ crumb.label }}</span>
              }
            }
          </nav>
        }

        @if (badge) {
          <div class="badge-tag">
            <span class="rune">ᛟ</span>
            <span>{{ badge }}</span>
          </div>
        }

        <h1 class="page-title">{{ title }}</h1>

        <div class="header-flourish-wrap">
          <img src="assets/images/rune-divider-flourish.svg" alt="" class="header-flourish-svg" />
        </div>

        @if (subtitle) {
          <p class="page-subtitle">{{ subtitle }}</p>
        }

        <ng-content></ng-content>
      </div>
    </header>
  `,
  styleUrls: ['./page-header.component.scss']
})
export class PageHeaderComponent {
  @Input({ required: true }) title!: string;
  @Input() subtitle?: string;
  @Input() badge?: string;
  @Input() bgImage?: string;
  @Input() breadcrumbs?: { label: string; route?: string | any[] }[];
}
