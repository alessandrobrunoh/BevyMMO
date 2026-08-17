import { Component, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';

export type SlotType = 'essence' | 'modifier' | 'ancient-word';

@Component({
  selector: 'app-rune-slot',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div
      class="rune-slot-socket chamfer-box"
      [ngClass]="['type-' + type, active ? 'is-active' : '']"
      (click)="selectSlot.emit()"
    >
      <div class="socket-rim">
        <span class="slot-rune">{{ runeGlyph }}</span>
      </div>
      <div class="socket-info">
        <span class="slot-type-label">{{ label }}</span>
        <h5 class="slot-value">{{ value || 'Empty Slot' }}</h5>
      </div>
      @if (active) {
        <span class="active-indicator">⚡</span>
      }
    </div>
  `,
  styleUrls: ['./rune-slot.component.scss']
})
export class RuneSlotComponent {
  @Input() type: SlotType = 'essence';
  @Input() label = 'Essence';
  @Input() value = '';
  @Input() runeGlyph = 'ᚠ';
  @Input() active = false;

  @Output() selectSlot = new EventEmitter<void>();
}
