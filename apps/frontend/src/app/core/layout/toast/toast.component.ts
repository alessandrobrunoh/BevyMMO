import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { ToastService, ToastMessage } from '../../services/toast.service';

@Component({
  selector: 'app-toast-container',
  standalone: true,
  imports: [CommonModule, EivarButtonComponent],
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
              @case ('success') { <span class="material-symbols-outlined">check_circle</span> }
              @case ('warning') { <span class="material-symbols-outlined">warning</span> }
              @case ('rune') { <span class="rune-glyph">ᛟ</span> }
              @default { <span class="material-symbols-outlined">info</span> }
            }
          </div>
          <div class="toast-body">
            @if (toast.title) {
              <h5 class="toast-title">{{ toast.title }}</h5>
            }
            <p class="toast-msg">{{ toast.message }}</p>
          </div>
          <app-eivar-button variant="icon-square" class="toast-close" ariaLabel="Close notification" (onClick)="toastService.dismiss(toast.id); $event.stopPropagation()">
            <span class="material-symbols-outlined">close</span>
          </app-eivar-button>
        </div>
      }
    </div>
  `,
  styleUrls: ['./toast.component.scss']
})
export class ToastContainerComponent {
  toastService = inject(ToastService);
}
