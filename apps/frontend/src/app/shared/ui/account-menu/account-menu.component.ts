import { Component, inject, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { AuthMockService } from '../../../core/services/auth-mock.service';
import { ToastService } from '../../../core/services/toast.service';

@Component({
  selector: 'app-account-menu',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="account-menu-dropdown chamfer-box">
      <header class="menu-header">
        <div class="user-avatar">
          <span class="avatar-rune">ᛟ</span>
        </div>
        <div class="user-details">
          <h4 class="user-name">{{ user?.name || 'Wayfarer' }}</h4>
          <span class="user-rank">{{ user?.rank || 'Alpha Explorer' }}</span>
        </div>
      </header>

      <div class="user-currencies">
        <div class="currency-item">
          <span class="cur-icon">✦</span>
          <span class="cur-label">Astral Marks:</span>
          <span class="cur-val">{{ user?.astralMarks || 2400 }}</span>
        </div>
      </div>

      <nav class="menu-links">
        <button class="menu-link-btn" (click)="onAction('Profile')">
          <span class="material-symbols-outlined btn-icon">person</span>
          <span>Player Profile & Runes</span>
        </button>
        <button class="menu-link-btn" (click)="onAction('Account')">
          <span class="material-symbols-outlined btn-icon">shield</span>
          <span>Alpha Access & Security</span>
        </button>
        <button class="menu-link-btn" (click)="onAction('Settings')">
          <span class="material-symbols-outlined btn-icon">settings</span>
          <span>Game & Display Settings</span>
        </button>
        <button class="menu-link-btn logout-btn" (click)="onLogout()">
          <span class="material-symbols-outlined btn-icon">logout</span>
          <span>Sign Out</span>
        </button>
      </nav>
    </div>
  `,
  styleUrls: ['./account-menu.component.scss']
})
export class AccountMenuComponent {
  authService = inject(AuthMockService);
  toastService = inject(ToastService);
  router = inject(Router);

  @Input() user = this.authService.currentUser();
  @Output() closeMenu = new EventEmitter<void>();

  onAction(actionName: string) {
    this.toastService.showInfo(`${actionName} screen is for prototype demonstration.`, 'Eivar Account');
    this.closeMenu.emit();
  }

  onLogout() {
    this.authService.logoutMock();
    this.toastService.showSuccess('You have logged out of the Eivar prototype.', 'Signed Out');
    this.closeMenu.emit();
    this.router.navigate(['/']);
  }
}
