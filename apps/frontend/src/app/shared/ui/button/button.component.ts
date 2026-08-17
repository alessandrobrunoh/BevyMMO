import { Component, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';

export type ButtonVariant = 'primary' | 'secondary' | 'cyan' | 'gold' | 'ghost';
export type ButtonSize = 'sm' | 'md' | 'lg';

@Component({
  selector: 'app-eivar-button',
  standalone: true,
  imports: [CommonModule],
  template: `
    <button
      [type]="type"
      [disabled]="disabled || loading"
      [ngClass]="[
        'eivar-btn',
        'variant-' + variant,
        'size-' + size,
        fullWidth ? 'full-width' : ''
      ]"
      (click)="onClick.emit($event)"
    >
      <!-- Active ambient glow layer -->
      <span class="glow-layer"></span>

      <!-- Shimmer light sweep -->
      <span class="shimmer-layer"></span>

      <!-- Button Content -->
      <span class="btn-content">
        @if (loading) {
          <span class="spinner-rune">ᛟ</span>
        }
        @if (icon) {
          <span class="btn-icon">{{ icon }}</span>
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
  @Input() icon?: string;

  @Output() onClick = new EventEmitter<MouseEvent>();
}

