import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ToastService, ToastMessage } from '../../services/toast.service';

@Component({
  selector: 'app-toast-container',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="toast-wrapper" aria-live="polite">
      @for (toast of toastService.toasts(); track toast.id) {
        <div
          class="toast-item chamfer-box"
          [ngClass]="'toast-' + toast.type"
          (click)="toastService.dismiss(toast.id)"
        >
          <div class="toast-icon">
            @switch (toast.type) {
              @case ('success') { <span>✓</span> }
              @case ('warning') { <span>⚠</span> }
              @case ('rune') { <span class="rune-glyph">ᛟ</span> }
              @default { <span>ℹ</span> }
            }
          </div>
          <div class="toast-body">
            @if (toast.title) {
              <h5 class="toast-title">{{ toast.title }}</h5>
            }
            <p class="toast-msg">{{ toast.message }}</p>
          </div>
          <button class="toast-close" (click)="toastService.dismiss(toast.id); $event.stopPropagation()">
            ✕
          </button>
        </div>
      }
    </div>
  `,
  styleUrls: ['./toast.component.scss']
})
export class ToastContainerComponent {
  toastService = inject(ToastService);
}
