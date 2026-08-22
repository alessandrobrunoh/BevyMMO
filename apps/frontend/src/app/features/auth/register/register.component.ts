import { Component, inject, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, RouterModule } from '@angular/router';
import { AuthService } from '../../../core/services/auth.service';
import { ToastService } from '../../../core/services/toast.service';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { AuthShellComponent } from '../../../shared/ui/auth-shell/auth-shell.component';

@Component({
  selector: 'app-register',
  standalone: true,
  imports: [FormsModule, RouterModule, EivarButtonComponent, AuthShellComponent],
  templateUrl: './register.component.html',
  styleUrls: ['../login.component.scss']
})
export class RegisterComponent {
  private authService = inject(AuthService);
  private toastService = inject(ToastService);
  private router = inject(Router);

  readonly email = signal('');
  readonly password = signal('');
  readonly passwordConfirmation = signal('');
  readonly showPassword = signal(false);
  readonly isLoading = signal(false);
  readonly errorMessage = signal<string | null>(null);

  toggleShowPassword(): void {
    this.showPassword.update(value => !value);
  }

  async onSubmit(): Promise<void> {
    const email = this.email().trim();
    const password = this.password();

    if (!email.includes('@') || !email.includes('.')) {
      this.errorMessage.set('Enter a valid email address to begin.');
      return;
    }
    if (password.length < 8) {
      this.errorMessage.set('Choose a password with at least 8 characters.');
      return;
    }
    if (password !== this.passwordConfirmation()) {
      this.errorMessage.set('The two password glyphs do not match.');
      return;
    }

    this.errorMessage.set(null);
    this.isLoading.set(true);
    try {
      await this.authService.register(email, password);
      this.toastService.showSuccess(`Your account is ready, ${email.split('@')[0]}.`, 'Vanguard attunement');
      await this.router.navigateByUrl('/');
    } catch (error) {
      this.errorMessage.set(error instanceof Error ? error.message : 'Unable to forge your account right now.');
    } finally {
      this.isLoading.set(false);
    }
  }
}
