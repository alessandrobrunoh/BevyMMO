import { CommonModule } from '@angular/common';
import { Component, HostListener, inject, signal } from '@angular/core';
import { RouterModule } from '@angular/router';
import { AccountMenuComponent } from '../../../shared/ui/account-menu/account-menu.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { AuthService } from '../../services/auth.service';
import { SearchService } from '../../services/search.service';
import { ToastService } from '../../services/toast.service';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [CommonModule, RouterModule, AccountMenuComponent, EivarButtonComponent],
  template: `
    <header class="navbar" [class.scrolled]="isScrolled()">
      <div class="ornamental-shell">
        <picture class="header-frame" aria-hidden="true">
          <source srcset="/assets/images/header/header-frame.webp?v=2" type="image/webp" />
          <img
            src="/assets/images/header/header-frame.png?v=2"
            alt=""
            width="1880"
            height="180"
            decoding="async"
            fetchpriority="high"
          />
        </picture>

        <div class="nav-inner">
          <a routerLink="/" class="brand-link" aria-label="Eivar Online, home" (click)="closeMobileMenu()">
            <span class="brand-text">
              <img src="assets/branding/eivar-wordmark.svg" alt="Eivar" class="brand-wordmark" width="1185" height="254" />
              <span class="brand-subtitle">Online</span>
            </span>
          </a>

          <nav class="desktop-nav" aria-label="Primary navigation">
            <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{ exact: true }" class="nav-link">Game</a>
            <span class="nav-sep" aria-hidden="true">◆</span>
            <a routerLink="/news" routerLinkActive="active" class="nav-link">News</a>
            <span class="nav-sep" aria-hidden="true">◆</span>
            <a routerLink="/updates" routerLinkActive="active" class="nav-link">Updates</a>
            <span class="nav-sep" aria-hidden="true">◆</span>
            <a routerLink="/wiki" routerLinkActive="active" class="nav-link">Wiki</a>
            <span class="nav-sep" aria-hidden="true">◆</span>
            <a routerLink="/store" routerLinkActive="active" class="nav-link">Store</a>
            <span class="nav-sep" aria-hidden="true">◆</span>
            <a routerLink="/market" routerLinkActive="active" class="nav-link">Market</a>
          </nav>

          <div class="nav-actions">
            <app-eivar-button variant="icon-square" class="nav-action" [iconOnly]="true" ariaLabel="Search" (onClick)="searchService.open()">
              <svg class="nav-ico" viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="10.8" cy="10.8" r="6.3"></circle>
                <path d="m15.5 15.5 4.2 4.2"></path>
              </svg>
            </app-eivar-button>

            <app-eivar-button variant="icon-square" class="nav-action" [iconOnly]="true" ariaLabel="Community" (onClick)="onCommunityClick()">
              <svg class="nav-ico" viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="9" cy="8" r="3"></circle>
                <circle cx="17" cy="9" r="2.3"></circle>
                <path d="M3.5 19c.4-3.6 2.2-5.4 5.5-5.4s5.1 1.8 5.5 5.4M14.2 14.4c3.8-.7 5.8.8 6.3 4.1"></path>
              </svg>
            </app-eivar-button>

            @if (authService.isLoggedIn()) {
              <div class="account-wrapper">
                <app-eivar-button
                  variant="icon-square"
                  class="nav-action"
                  [iconOnly]="true"
                  ariaLabel="Open account menu"
                  [ariaExpanded]="isAccountMenuOpen()"
                  (onClick)="toggleAccountMenu()"
                >
                  <svg class="nav-ico" viewBox="0 0 24 24" aria-hidden="true">
                    <circle cx="12" cy="8" r="3.4"></circle>
                    <path d="M5.2 20c.5-4.2 2.8-6.3 6.8-6.3s6.3 2.1 6.8 6.3"></path>
                  </svg>
                </app-eivar-button>
                @if (isAccountMenuOpen()) {
                  <app-account-menu (closeMenu)="isAccountMenuOpen.set(false)"></app-account-menu>
                }
              </div>
            } @else {
              <app-eivar-button variant="icon-square" routerLink="/login" class="nav-action" [iconOnly]="true" ariaLabel="Login">
                <svg class="nav-ico" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M13.5 5H19v14h-5.5M4 12h10M10.5 8.5 14 12l-3.5 3.5"></path>
                </svg>
              </app-eivar-button>
            }

            <app-eivar-button
              variant="icon-circle"
              [tone]="isMobileMenuOpen() ? 'red' : 'blue'"
              class="mobile-menu-button"
              [iconOnly]="true"
              [ariaLabel]="isMobileMenuOpen() ? 'Close navigation menu' : 'Open navigation menu'"
              [ariaExpanded]="isMobileMenuOpen()"
              ariaControls="mobile-navigation"
              (onClick)="toggleMobileMenu()"
            >
              @if (isMobileMenuOpen()) {
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="m6 6 12 12M18 6 6 18"></path>
                </svg>
              } @else {
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M5 7h14M5 12h14M5 17h14"></path>
                </svg>
              }
            </app-eivar-button>
          </div>
        </div>

      </div>
    </header>

    <nav
      id="mobile-navigation"
      class="mobile-menu"
      [class.open]="isMobileMenuOpen()"
      [attr.aria-hidden]="!isMobileMenuOpen()"
      [attr.inert]="isMobileMenuOpen() ? null : ''"
      aria-label="Mobile navigation"
    >
      <ul>
        <li><a routerLink="/" (click)="closeMobileMenu()">Game</a></li>
        <li><a routerLink="/news" (click)="closeMobileMenu()">News</a></li>
        <li><a routerLink="/updates" (click)="closeMobileMenu()">Updates</a></li>
        <li><a routerLink="/wiki" (click)="closeMobileMenu()">Wiki</a></li>
        <li><a routerLink="/store" (click)="closeMobileMenu()">Store</a></li>
        <li><a routerLink="/market" (click)="closeMobileMenu()">Market</a></li>
        <li><app-eivar-button variant="navigation" [fullWidth]="true" (onClick)="searchService.open(); closeMobileMenu()">Search</app-eivar-button></li>
        <li><app-eivar-button variant="navigation" [fullWidth]="true" (onClick)="onCommunityClick(); closeMobileMenu()">Community</app-eivar-button></li>
        @if (authService.isLoggedIn()) {
          <li><app-eivar-button variant="danger" [fullWidth]="true" (onClick)="authService.logout(); closeMobileMenu()">Logout</app-eivar-button></li>
        } @else {
          <li><a routerLink="/login" (click)="closeMobileMenu()">Login</a></li>
        }
      </ul>
    </nav>
  `,
  styleUrls: ['./header.component.scss'],
})
export class HeaderComponent {
  searchService = inject(SearchService);
  authService = inject(AuthService);
  toastService = inject(ToastService);

  isScrolled = signal(false);
  isMobileMenuOpen = signal(false);
  isAccountMenuOpen = signal(false);

  @HostListener('window:scroll')
  onWindowScroll(): void {
    this.isScrolled.set(window.scrollY > 20);
  }

  @HostListener('document:keydown', ['$event'])
  onGlobalKeyDown(event: KeyboardEvent): void {
    if ((event.metaKey || event.ctrlKey) && event.key === 'k') {
      event.preventDefault();
      this.searchService.open();
    }

    if (event.key === 'Escape') {
      this.closeMobileMenu();
      this.isAccountMenuOpen.set(false);
    }
  }

  toggleMobileMenu(): void {
    this.isMobileMenuOpen.update((open) => !open);
  }

  closeMobileMenu(): void {
    this.isMobileMenuOpen.set(false);
  }

  toggleAccountMenu(): void {
    this.isAccountMenuOpen.update((open) => !open);
  }

  onCommunityClick(): void {
    this.toastService.showInfo('Official Eivar Discord & Community Guilds link active.', 'Eivar Community');
  }
}
