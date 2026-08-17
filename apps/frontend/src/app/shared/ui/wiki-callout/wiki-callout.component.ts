import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-wiki-callout',
  standalone: true,
  imports: [CommonModule],
  template: `
    <aside class="wiki-callout" [ngClass]="'type-' + type">
      <div class="callout-icon">
        @switch (type) {
          @case ('tip') { <span class="material-symbols-outlined">auto_awesome</span> }
          @case ('warning') { <span class="material-symbols-outlined">warning</span> }
          @default { <span class="rune">ᛟ</span> }
        }
      </div>
      <div class="callout-content">
        @if (title) {
          <h5 class="callout-title">{{ title }}</h5>
        }
        <p class="callout-text">
          <ng-content></ng-content>
          @if (text) { {{ text }} }
        </p>
      </div>
    </aside>
  `,
  styleUrls: ['./wiki-callout.component.scss']
})
export class WikiCalloutComponent {
  @Input() type: 'info' | 'warning' | 'tip' = 'info';
  @Input() title?: string;
  @Input() text?: string;
}
