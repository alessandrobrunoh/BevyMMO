import { Component, Input } from '@angular/core';

export type AuthShellMode = 'login' | 'register';

@Component({
  selector: 'app-auth-shell',
  standalone: true,
  template: `
    <section class="auth-shell" [class.auth-shell--register]="mode === 'register'">
      <div class="auth-shell__stage">
        <div class="auth-shell__portal">
          <img src="assets/images/auth-portal.png" alt="" aria-hidden="true" />

          <div class="auth-shell__content">
            <header class="auth-shell__header">
              <p>{{ mode === 'login' ? 'Vanguard access' : 'The first inscription' }}</p>
              <h1>{{ title }}</h1>
              <span class="auth-shell__divider" aria-hidden="true">◇</span>
            </header>

            <ng-content></ng-content>
          </div>
        </div>
      </div>
    </section>
  `,
  styleUrls: ['./auth-shell.component.scss']
})
export class AuthShellComponent {
  @Input({ required: true }) mode!: AuthShellMode;
  @Input({ required: true }) title!: string;
}
