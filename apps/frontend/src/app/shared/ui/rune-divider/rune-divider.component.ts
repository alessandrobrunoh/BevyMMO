import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-rune-divider',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="rune-divider" [ngClass]="['style-' + styleVariant, glow ? 'with-glow' : '']">
      <span class="line left-line"></span>
      <div class="rune-emblem">
        <svg viewBox="0 0 40 40" class="rune-svg" fill="none">
          <circle cx="20" cy="20" r="16" stroke="currentColor" stroke-width="1" stroke-dasharray="2 2" opacity="0.6"/>
          <path d="M20 6 L28 20 L20 34 L12 20 Z" stroke="currentColor" stroke-width="1.5"/>
          <circle cx="20" cy="20" r="2.5" fill="currentColor"/>
          <line x1="8" y1="20" x2="32" y2="20" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </div>
      <span class="line right-line"></span>
    </div>
  `,
  styleUrls: ['./rune-divider.component.scss']
})
export class RuneDividerComponent {
  @Input() styleVariant: 'gold' | 'cyan' | 'stone' = 'cyan';
  @Input() glow = true;
}
