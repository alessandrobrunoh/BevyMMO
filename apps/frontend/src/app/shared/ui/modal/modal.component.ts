import { Component, Input, Output, EventEmitter, HostListener } from '@angular/core';
import { CommonModule } from '@angular/common';
import { EivarButtonComponent } from '../button/button.component';

@Component({
  selector: 'app-modal',
  standalone: true,
  imports: [CommonModule, EivarButtonComponent],
  template: `
    @if (isOpen) {
      <div class="modal-backdrop" (click)="onBackdropClick($event)" role="dialog" aria-modal="true">
        <div class="modal-container chamfer-box">
          <header class="modal-header">
            <h3 class="modal-title">{{ title }}</h3>
            <app-eivar-button variant="icon-square" class="close-btn" ariaLabel="Close modal" (onClick)="close.emit()">
              <span class="material-symbols-outlined">close</span>
            </app-eivar-button>
          </header>
          <div class="modal-body">
            <ng-content></ng-content>
          </div>
          @if (showFooter) {
            <footer class="modal-footer">
              <ng-content select="[footer]"></ng-content>
            </footer>
          }
        </div>
      </div>
    }
  `,
  styleUrls: ['./modal.component.scss']
})
export class ModalComponent {
  @Input() isOpen = false;
  @Input() title = '';
  @Input() showFooter = false;

  @Output() close = new EventEmitter<void>();

  @HostListener('document:keydown.escape')
  onEscape() {
    if (this.isOpen) {
      this.close.emit();
    }
  }

  onBackdropClick(event: MouseEvent) {
    if ((event.target as HTMLElement).classList.contains('modal-backdrop')) {
      this.close.emit();
    }
  }
}
