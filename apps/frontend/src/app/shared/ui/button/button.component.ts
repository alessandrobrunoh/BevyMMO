import { CommonModule } from '@angular/common';
import { Component, EventEmitter, HostBinding, Input, Output } from '@angular/core';
import { Params, RouterLink, UrlTree } from '@angular/router';

export type ButtonVariant =
  | 'primary'
  | 'secondary'
  | 'cyan'
  | 'info'
  | 'success'
  | 'danger'
  | 'ornate'
  | 'gold'
  | 'ghost'
  | 'navigation'
  | 'outline'
  | 'tag'
  | 'cta'
  | 'icon-square'
  | 'icon-circle'
  | 'arrow-left'
  | 'arrow-right'
  | 'social';
export type ButtonSize = 'sm' | 'md' | 'lg';
export type ButtonTone = 'blue' | 'green' | 'red' | 'gold';

@Component({
  selector: 'app-eivar-button',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <ng-template #buttonContent>
      <span class="eivar-btn__content">
        @if (loading) {
          <span class="eivar-btn__spinner" aria-hidden="true">✦</span>
        } @else if (icon) {
          <span
            [class.material-symbols-outlined]="iconSet === 'material'"
            class="eivar-btn__icon"
            aria-hidden="true"
          >{{ icon }}</span>
        }
        <ng-content></ng-content>
      </span>
    </ng-template>

    @if (routerLink !== null) {
      <a
        [routerLink]="routerLink"
        [queryParams]="queryParams"
        [fragment]="fragment"
        [attr.aria-label]="ariaLabel || null"
        [attr.aria-current]="ariaCurrent || null"
        [attr.aria-disabled]="disabled || loading ? 'true' : null"
        [attr.aria-busy]="loading ? 'true' : null"
        [attr.tabindex]="disabled || loading ? -1 : null"
        [ngClass]="buttonClasses"
        (click)="handleClick($event)"
      >
        <ng-container *ngTemplateOutlet="buttonContent"></ng-container>
      </a>
    } @else if (href) {
      <a
        [href]="href"
        [target]="target || null"
        [rel]="rel || null"
        [attr.download]="download || null"
        [attr.aria-label]="ariaLabel || null"
        [attr.aria-disabled]="disabled || loading ? 'true' : null"
        [attr.aria-busy]="loading ? 'true' : null"
        [attr.tabindex]="disabled || loading ? -1 : null"
        [ngClass]="buttonClasses"
        (click)="handleClick($event)"
      >
        <ng-container *ngTemplateOutlet="buttonContent"></ng-container>
      </a>
    } @else {
      <button
        [type]="type"
        [disabled]="disabled || loading"
        [attr.aria-label]="ariaLabel || null"
        [attr.aria-pressed]="toggle ? active : null"
        [attr.aria-expanded]="ariaExpanded"
        [attr.aria-controls]="ariaControls || null"
        [attr.aria-busy]="loading ? 'true' : null"
        [ngClass]="buttonClasses"
        (click)="handleClick($event)"
      >
        <ng-container *ngTemplateOutlet="buttonContent"></ng-container>
      </button>
    }
  `,
  styleUrls: ['./button.component.scss']
})
export class EivarButtonComponent {
  @Input() variant: ButtonVariant = 'primary';
  @Input() size: ButtonSize = 'md';
  @Input() tone: ButtonTone = 'blue';
  @Input() type: 'button' | 'submit' | 'reset' = 'button';
  @Input() disabled = false;
  @Input() loading = false;
  @Input() fullWidth = false;
  @Input() active = false;
  @Input() toggle = false;
  @Input() iconOnly = false;
  @Input() icon?: string;
  @Input() iconSet: 'material' | 'glyph' = 'material';
  @Input() ariaLabel?: string;
  @Input() ariaExpanded: boolean | null = null;
  @Input() ariaControls?: string;
  @Input() ariaCurrent?: 'page' | 'step' | 'location' | 'date' | 'time' | 'true';
  @Input() routerLink: string | readonly (string | number)[] | UrlTree | null = null;
  @Input() queryParams: Params | null = null;
  @Input() fragment?: string;
  @Input() href?: string;
  @Input() target?: '_self' | '_blank' | '_parent' | '_top';
  @Input() rel?: string;
  @Input() download?: string;

  @Output() onClick = new EventEmitter<MouseEvent>();

  @HostBinding('class.full-width')
  get hasFullWidth(): boolean {
    return this.fullWidth;
  }

  get buttonClasses(): string[] {
    return [
      'eivar-btn',
      `variant-${this.variant}`,
      `size-${this.size}`,
      `tone-${this.tone}`,
      this.fullWidth ? 'full-width' : '',
      this.active ? 'is-active' : '',
      this.iconOnly ? 'icon-only' : ''
    ];
  }

  handleClick(event: MouseEvent): void {
    if (this.disabled || this.loading) {
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }

    this.onClick.emit(event);
  }
}
