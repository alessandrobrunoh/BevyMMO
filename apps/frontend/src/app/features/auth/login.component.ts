import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
import { AuthService } from '../../core/services/auth.service';
import { ToastService } from '../../core/services/toast.service';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { RuneDividerComponent } from '../../shared/ui/rune-divider/rune-divider.component';

export type AuthMode = 'login' | 'register' | 'forgot';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule, EivarButtonComponent, RuneDividerComponent],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.scss']
})
export class LoginComponent {
  private authService = inject(AuthService);
  private toastService = inject(ToastService);
  private router = inject(Router);

  readonly mode = signal<AuthMode>('login');
  readonly email = signal<string>('wayfarer@eivar.online');
  readonly password = signal<string>('alpha2026');
  readonly rememberMe = signal<boolean>(true);
  readonly showPassword = signal<boolean>(false);
  readonly isLoading = signal<boolean>(false);
  readonly errorMessage = signal<string | null>(null);

  setMode(newMode: AuthMode) {
    this.mode.set(newMode);
    this.errorMessage.set(null);
  }

  toggleShowPassword() {
    this.showPassword.update(v => !v);
  }

  async onSubmit() {
    const em = this.email().trim();
    const pw = this.password().trim();

    // Local format validation — the gateway/module re-validate authoritatively
    // (reducers::account::{validate_email,validate_password}); this is only
    // to avoid a round trip for the obviously-wrong cases.
    if (!em || !em.includes('@') || !em.includes('.')) {
      this.errorMessage.set('Please enter a valid email address (e.g. name@domain.com).');
      return;
    }

    if (this.mode() !== 'forgot' && (!pw || pw.length < 8)) {
      this.errorMessage.set('Password must contain at least 8 characters.');
      return;
    }

    this.errorMessage.set(null);

    if (this.mode() === 'forgot') {
      this.toastService.showSuccess(
        `Password recovery dispatched to ${em} (Prototype demo).`,
        'Recovery Rune'
      );
      this.setMode('login');
      return;
    }

    this.isLoading.set(true);
    try {
      if (this.mode() === 'register') {
        await this.authService.register(em, pw);
      } else {
        await this.authService.login(em, pw);
      }
      this.toastService.showSuccess(
        `Welcome to Eivar Online, ${em.split('@')[0]}!`,
        'Vanguard Attunement'
      );
      this.router.navigate(['/']);
    } catch (err) {
      this.errorMessage.set(err instanceof Error ? err.message : 'Something went wrong.');
    } finally {
      this.isLoading.set(false);
    }
  }
}
