import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { AuthService } from '../../core/services/auth.service';
import { ToastService } from '../../core/services/toast.service';
import { EivarButtonComponent } from '../../shared/ui/button/button.component';
import { AuthShellComponent } from '../../shared/ui/auth-shell/auth-shell.component';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [FormsModule, RouterModule, EivarButtonComponent, AuthShellComponent],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.scss']
})
export class LoginComponent {
  private authService = inject(AuthService);
  private toastService = inject(ToastService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);

  readonly email = signal('');
  readonly password = signal('');
  readonly rememberMe = signal(true);
  readonly showPassword = signal(false);
  readonly isLoading = signal(false);
  readonly errorMessage = signal<string | null>(null);

  toggleShowPassword(): void {
    this.showPassword.update(value => !value);
  }

  async onSubmit(): Promise<void> {
    const email = this.email().trim();
    const password = this.password();

    if (!isValidEmail(email)) {
      this.errorMessage.set('Enter a valid email address to continue.');
      return;
    }
    if (password.length < 8) {
      this.errorMessage.set('Your password must contain at least 8 characters.');
      return;
    }

    this.errorMessage.set(null);
    this.isLoading.set(true);
    try {
      await this.authService.login(email, password);
      this.toastService.showSuccess(`Welcome back, ${email.split('@')[0]}.`, 'Attunement restored');
      await this.router.navigateByUrl(safeReturnUrl(this.route.snapshot.queryParamMap.get('returnUrl')));
    } catch (error) {
      this.errorMessage.set(error instanceof Error ? error.message : 'Unable to enter Eivar right now.');
    } finally {
      this.isLoading.set(false);
    }
  }
}

function isValidEmail(value: string): boolean {
  return value.includes('@') && value.includes('.');
}

function safeReturnUrl(raw: string | null): string {
  return raw && raw.startsWith('/') && !raw.startsWith('//') ? raw : '/';
}
