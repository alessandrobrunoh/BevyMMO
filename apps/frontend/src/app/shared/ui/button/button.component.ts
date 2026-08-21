import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';

export type ButtonVariant =
  | 'primary'
  | 'secondary'
  | 'cyan'
  | 'ornate'
  | 'gold'
  | 'ghost'
  | 'navigation'
  | 'outline'
  | 'tag'
  | 'cta'
  | 'icon-square'
  | 'icon-circle'
  | 'social';
export type ButtonSize = 'sm' | 'md' | 'lg';

@Component({
  selector: 'app-eivar-button',
  standalone: true,
  imports: [CommonModule],
  template: `
    <button
      [type]="type"
      [disabled]="disabled || loading"
      [attr.aria-label]="ariaLabel || null"
      [attr.aria-pressed]="toggle ? active : null"
      [ngClass]="[
        'eivar-btn',
        'variant-' + variant,
        'size-' + size,
        fullWidth ? 'full-width' : '',
        active ? 'is-active' : '',
        iconOnly ? 'icon-only' : ''
      ]"
      (click)="onClick.emit($event)"
    >
      <span class="eivar-btn__metal" aria-hidden="true"></span>
      <span class="eivar-btn__rune" aria-hidden="true">✦</span>
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
    </button>
  `,
  styleUrls: ['./button.component.scss']
})
export class EivarButtonComponent {
  @Input() variant: ButtonVariant = 'primary';
  @Input() size: ButtonSize = 'md';
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

  @Output() onClick = new EventEmitter<MouseEvent>();
}
