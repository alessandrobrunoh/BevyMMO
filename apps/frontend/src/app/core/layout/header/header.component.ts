import { Component, inject, HostListener, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router } from '@angular/router';
import { SearchService } from '../../services/search.service';
import { AuthMockService } from '../../services/auth-mock.service';
import { ToastService } from '../../services/toast.service';
import { AccountMenuComponent } from '../../../shared/ui/account-menu/account-menu.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [CommonModule, RouterModule, AccountMenuComponent, EivarButtonComponent],
  template: `
    <header class="global-navbar" [class.scrolled]="isScrolled()" [class.menu-open]="isMobileMenuOpen()">
      <div class="container nav-inner">
        <!-- Brand Logo -->
        <a routerLink="/" class="nav-brand" (click)="closeMobileMenu()">
          <div class="brand-emblem">
            <svg viewBox="0 0 100 160" class="brand-rune-svg" fill="none">
              <!-- Top Diamond -->
              <path d="M50 14 L54 30 L50 26 L46 30 Z" fill="#3ccbff" />
              <line x1="50" y1="14" x2="50" y2="40" stroke="#3ccbff" stroke-width="3" stroke-linecap="round" />
              <path d="M50 35 L68 56 L50 78 L32 56 Z" stroke="#3ccbff" stroke-width="3.5" fill="none" stroke-linejoin="round" />
              <line x1="24" y1="56" x2="76" y2="56" stroke="#3ccbff" stroke-width="3" stroke-linecap="round" />
              <circle cx="50" cy="80" r="4" fill="#e6cb86" />
              <path d="M50 82 L68 104 L50 126 L32 104 Z" stroke="#3ccbff" stroke-width="3.5" fill="none" stroke-linejoin="round" />
              <line x1="24" y1="104" x2="76" y2="104" stroke="#3ccbff" stroke-width="3" stroke-linecap="round" />
              <line x1="50" y1="120" x2="50" y2="146" stroke="#3ccbff" stroke-width="3" stroke-linecap="round" />
            </svg>
          </div>
          <div class="brand-typography">
            <span class="brand-name">EIVAR</span>
            <span class="brand-online">ONLINE</span>
          </div>
        </a>

        <!-- Desktop Navigation Links -->
        <nav class="desktop-nav-links">
          <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{exact: true}" class="nav-link">
            <span class="link-rune">ᛟ</span>
            <span>Game</span>
          </a>
          <a routerLink="/news" routerLinkActive="active" class="nav-link">
            <span class="link-rune">ᚱ</span>
            <span>News</span>
          </a>
          <a routerLink="/updates" routerLinkActive="active" class="nav-link">
            <span class="link-rune">ᛉ</span>
            <span>Updates</span>
          </a>
          <a routerLink="/wiki" routerLinkActive="active" class="nav-link">
            <span class="link-rune">ᚹ</span>
            <span>Wiki</span>
          </a>
          <a routerLink="/store" routerLinkActive="active" class="nav-link">
            <span class="link-rune">ᛏ</span>
            <span>Store</span>
          </a>
        </nav>

        <!-- Right Utilities -->
        <div class="nav-utilities">
          <!-- Search Trigger -->
          <button class="util-btn search-btn" (click)="searchService.open()" title="Search (Cmd+K)">
            <span class="icon">🔍</span>
            <span class="btn-text">Search</span>
            <span class="key-hint">⌘K</span>
          </button>

          <!-- Community Button -->
          <button class="util-btn" (click)="onCommunityClick()" title="Community Discord">
            <span class="icon">💬</span>
            <span class="btn-text">Community</span>
          </button>

          <!-- Auth & Account State -->
          @if (authService.isLoggedIn()) {
            <div class="account-wrapper">
              <button class="account-badge-btn" (click)="toggleAccountMenu()">
                <div class="badge-avatar">
                  <span>ᛟ</span>
                </div>
                <span class="badge-name">{{ authService.currentUser()?.name }}</span>
                <span class="arrow-down">▾</span>
              </button>
              @if (isAccountMenuOpen()) {
                <app-account-menu (closeMenu)="isAccountMenuOpen.set(false)"></app-account-menu>
              }
            </div>
          } @else {
            <a routerLink="/login" class="login-link">
              <span class="icon">🛡️</span>
              <span>Login</span>
            </a>
          }

          <!-- Play Alpha Primary CTA -->
          <app-eivar-button variant="cyan" size="sm" (onClick)="onPlayClick()">
            <span>Play Alpha</span>
          </app-eivar-button>

          <!-- Hamburger Toggle for Mobile -->
          <button class="mobile-toggle-btn" (click)="toggleMobileMenu()" aria-label="Toggle navigation menu">
            <span class="bar bar-1"></span>
            <span class="bar bar-2"></span>
            <span class="bar bar-3"></span>
          </button>
        </div>
      </div>

      <!-- Mobile Full-Screen Drawer -->
      @if (isMobileMenuOpen()) {
        <div class="mobile-drawer anim-fade-in">
          <nav class="mobile-nav-links">
            <a routerLink="/" (click)="closeMobileMenu()" class="mobile-nav-item">
              <span class="mob-rune">ᛟ</span>
              <span>Game & Lore</span>
            </a>
            <a routerLink="/news" (click)="closeMobileMenu()" class="mobile-nav-item">
              <span class="mob-rune">ᚱ</span>
              <span>News & Articles</span>
            </a>
            <a routerLink="/updates" (click)="closeMobileMenu()" class="mobile-nav-item">
              <span class="mob-rune">ᛉ</span>
              <span>Patch Notes & Updates</span>
            </a>
            <a routerLink="/wiki" (click)="closeMobileMenu()" class="mobile-nav-item">
              <span class="mob-rune">ᚹ</span>
              <span>The Eivar Archives (Wiki)</span>
            </a>
            <a routerLink="/store" (click)="closeMobileMenu()" class="mobile-nav-item">
              <span class="mob-rune">ᛏ</span>
              <span>Cosmetics & Supporter Store</span>
            </a>
            @if (authService.isLoggedIn()) {
              <div class="mobile-user-card">
                <span class="user-greeting">Logged in as {{ authService.currentUser()?.name }}</span>
                <button class="mobile-logout" (click)="authService.logoutMock(); closeMobileMenu()">Sign Out</button>
              </div>
            } @else {
              <a routerLink="/login" (click)="closeMobileMenu()" class="mobile-nav-item auth-item">
                <span class="mob-rune">🛡️</span>
                <span>Account Login / Sign Up</span>
              </a>
            }
          </nav>
          <div class="mobile-drawer-footer">
            <app-eivar-button variant="cyan" size="lg" [fullWidth]="true" (onClick)="onPlayClick(); closeMobileMenu()">
              Join Alpha Playtest
            </app-eivar-button>
          </div>
        </div>
      }
    </header>
  `,
  styleUrls: ['./header.component.scss']
})
export class HeaderComponent {
  searchService = inject(SearchService);
  authService = inject(AuthMockService);
  toastService = inject(ToastService);
  router = inject(Router);

  isScrolled = signal<boolean>(false);
  isMobileMenuOpen = signal<boolean>(false);
  isAccountMenuOpen = signal<boolean>(false);

  @HostListener('window:scroll', [])
  onWindowScroll() {
    this.isScrolled.set(window.scrollY > 40);
  }

  @HostListener('document:keydown', ['$event'])
  onGlobalKeyDown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === 'k') {
      event.preventDefault();
      this.searchService.open();
    }
  }

  toggleMobileMenu() {
    this.isMobileMenuOpen.update(v => !v);
  }

  closeMobileMenu() {
    this.isMobileMenuOpen.set(false);
  }

  toggleAccountMenu() {
    this.isAccountMenuOpen.update(v => !v);
  }

  onCommunityClick() {
    this.toastService.showInfo('Official Discord & Community Guilds link active.', 'Eivar Community');
  }

  onPlayClick() {
    if (!this.authService.isLoggedIn()) {
      this.toastService.showRunic('Alpha testing build available. Sign in or explore the archives!', 'Eivar Alpha');
      this.router.navigate(['/login']);
    } else {
      this.toastService.showSuccess('Welcome back, Wayfarer! Client synchronization initialized.', 'Alpha Connected');
    }
  }
}
