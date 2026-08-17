import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
import { AuthMockService } from '../../core/services/auth-mock.service';
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
  private authService = inject(AuthMockService);
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

  onSubmit() {
    const em = this.email().trim();
    const pw = this.password().trim();

    // Local Format Validation
    if (!em || !em.includes('@') || !em.includes('.')) {
      this.errorMessage.set('Please enter a valid email address (e.g. name@domain.com).');
      return;
    }

    if (this.mode() !== 'forgot' && (!pw || pw.length < 6)) {
      this.errorMessage.set('Password must contain at least 6 characters.');
      return;
    }

    this.errorMessage.set(null);
    this.isLoading.set(true);

    // Simulate mock network response
    setTimeout(() => {
      this.isLoading.set(false);

      if (this.mode() === 'forgot') {
        this.toastService.showSuccess(
          `Password recovery dispatched to ${em} (Prototype demo).`,
          'Recovery Rune'
        );
        this.setMode('login');
        return;
      }

      this.authService.loginMock(em);
      this.toastService.showSuccess(
        `Welcome to Eivar Online, ${em.split('@')[0]}!`,
        'Vanguard Attunement'
      );
      this.router.navigate(['/']);
    }, 1200);
  }
}
