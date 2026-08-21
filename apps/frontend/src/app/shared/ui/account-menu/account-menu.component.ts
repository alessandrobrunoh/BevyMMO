import { Component, inject, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { EivarButtonComponent } from '../button/button.component';
import { AuthService } from '../../../core/services/auth.service';
import { ToastService } from '../../../core/services/toast.service';

@Component({
  selector: 'app-account-menu',
  standalone: true,
  imports: [CommonModule, EivarButtonComponent],
  template: `
    <div class="account-menu-dropdown chamfer-box">
      <header class="menu-header">
        <div class="user-avatar">
          <span class="avatar-rune">ᛟ</span>
        </div>
        <div class="user-details">
          <h4 class="user-name">{{ authService.email() ?? 'Wayfarer' }}</h4>
          <span class="user-rank">Alpha Explorer</span>
        </div>
      </header>

      <div class="user-currencies">
        <div class="currency-item">
          <span class="cur-icon">✦</span>
          <span class="cur-label">Characters:</span>
          <span class="cur-val">{{ authService.profile()?.characters?.length ?? 0 }}</span>
        </div>
      </div>

      <nav class="menu-links">
        <app-eivar-button variant="navigation" [fullWidth]="true" icon="person" (onClick)="onAction('Profile')">
          <span>Player Profile & Runes</span>
        </app-eivar-button>
        <app-eivar-button variant="navigation" [fullWidth]="true" icon="shield" (onClick)="onAction('Account')">
          <span>Alpha Access & Security</span>
        </app-eivar-button>
        <app-eivar-button variant="navigation" [fullWidth]="true" icon="settings" (onClick)="onAction('Settings')">
          <span>Game & Display Settings</span>
        </app-eivar-button>
        <app-eivar-button variant="danger" class="logout-btn" [fullWidth]="true" icon="logout" (onClick)="onLogout()">
          <span>Sign Out</span>
        </app-eivar-button>
      </nav>
    </div>
  `,
  styleUrls: ['./account-menu.component.scss']
})
export class AccountMenuComponent {
  authService = inject(AuthService);
  toastService = inject(ToastService);
  router = inject(Router);

  @Output() closeMenu = new EventEmitter<void>();

  onAction(actionName: string) {
    if (actionName === 'Profile') {
      this.closeMenu.emit();
      this.router.navigate(['/profile']);
      return;
    }
    if (actionName === 'Account') {
      this.closeMenu.emit();
      this.router.navigate(['/account/api-keys']);
      return;
    }
    this.toastService.showInfo(`${actionName} screen is for prototype demonstration.`, 'Eivar Account');
    this.closeMenu.emit();
  }

  async onLogout() {
    await this.authService.logout();
    this.toastService.showSuccess('You have logged out of the Eivar prototype.', 'Signed Out');
    this.closeMenu.emit();
    this.router.navigate(['/']);
  }
}
