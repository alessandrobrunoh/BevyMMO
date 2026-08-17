import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-rune-divider',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="rune-divider" [ngClass]="styleVariant">
      <span class="divider-diamond">◇</span>
    </div>
  `,
  styleUrls: ['./rune-divider.component.scss']
})
export class RuneDividerComponent {
  @Input() styleVariant: 'gold' | 'cyan' | 'stone' = 'gold';
  @Input() glow = true;
}
