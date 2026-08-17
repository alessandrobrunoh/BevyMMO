import { Component, inject, HostListener, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router } from '@angular/router';
import { SearchService } from '../../services/search.service';
import { AuthService } from '../../services/auth.service';
import { ToastService } from '../../services/toast.service';
import { AccountMenuComponent } from '../../../shared/ui/account-menu/account-menu.component';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [CommonModule, RouterModule, AccountMenuComponent],
  template: `
    <header class="navbar" [class.scrolled]="isScrolled()">
      <div class="nav-inner">
        <!-- Brand Lockup: Seamless, frameless, glowing rune and vector wordmark -->
        <a routerLink="/" class="brand-link" (click)="closeMobileMenu()">
          <div class="brand-rune-wrap">
            <img src="assets/branding/eivar-rune.svg" alt="Eivar Emblem" class="brand-mark" />
            <span class="rune-glow-effect"></span>
          </div>
          <span class="brand-divider"></span>
          <span class="brand-text">
            <img src="assets/branding/eivar-wordmark.svg" alt="Eivar" class="brand-wordmark" />
            <span class="brand-subtitle">Online</span>
          </span>
        </a>

        <!-- Desktop Navigation Links -->
        <nav class="desktop-nav">
          <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{exact: true}" class="nav-link">
            <span>Game</span>
          </a>
          <span class="nav-sep">◈</span>
          <a routerLink="/news" routerLinkActive="active" class="nav-link">
            <span>News</span>
          </a>
          <span class="nav-sep">◈</span>
          <a routerLink="/updates" routerLinkActive="active" class="nav-link">
            <span>Updates</span>
          </a>
          <span class="nav-sep">◈</span>
          <a routerLink="/wiki" routerLinkActive="active" class="nav-link">
            <span>Wiki</span>
          </a>
          <span class="nav-sep">◈</span>
          <a routerLink="/store" routerLinkActive="active" class="nav-link">
            <span>Store</span>
          </a>
        </nav>

        <!-- Right Nav Actions Group -->
        <div class="nav-actions">
          <!-- Search Action -->
          <button class="nav-action search-btn" (click)="searchService.open()" title="Search (Cmd+K / Ctrl+K)">
            <span class="material-symbols-outlined nav-ico">search</span>
            <span class="action-label">Search</span>
            <kbd class="key-shortcut">⌘K</kbd>
          </button>

          <!-- Community Action -->
          <button class="nav-action" (click)="onCommunityClick()">
            <span class="material-symbols-outlined nav-ico">forum</span>
            <span class="action-label">Community</span>
          </button>

          <!-- Login / Account Action -->
          @if (authService.isLoggedIn()) {
            <div class="account-wrapper">
              <button class="nav-action account-btn" (click)="toggleAccountMenu()">
                <span class="material-symbols-outlined nav-ico">person</span>
                <span class="action-label">{{ authService.email() ?? 'Wayfarer' }}</span>
                <span class="material-symbols-outlined arrow-ico">expand_more</span>
              </button>
              @if (isAccountMenuOpen()) {
                <app-account-menu (closeMenu)="isAccountMenuOpen.set(false)"></app-account-menu>
              }
            </div>
          } @else {
            <a routerLink="/login" class="nav-action">
              <span class="material-symbols-outlined nav-ico">person</span>
              <span class="action-label">Login</span>
            </a>
          }

          <!-- Discover Eivar Button -->
          <a routerLink="/" fragment="world" class="cta-link">
            <button class="header-cta-button">
              <span class="btn-rune-spark">✦</span>
              <span class="btn-text">Discover Eivar</span>
            </button>
          </a>

          <!-- Mobile Hamburger Toggle -->
          <button
            class="mobile-menu-button"
            (click)="toggleMobileMenu()"
            aria-label="Open navigation menu"
          >
            <span class="material-symbols-outlined">menu</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Mobile Navigation Drawer -->
    <nav class="mobile-menu" [class.open]="isMobileMenuOpen()">
      <a routerLink="/" (click)="closeMobileMenu()">Game</a>
      <a routerLink="/news" (click)="closeMobileMenu()">News</a>
      <a routerLink="/updates" (click)="closeMobileMenu()">Updates</a>
      <a routerLink="/wiki" (click)="closeMobileMenu()">Wiki</a>
      <a routerLink="/store" (click)="closeMobileMenu()">Store</a>
      <a (click)="onCommunityClick(); closeMobileMenu()">Community</a>
      @if (authService.isLoggedIn()) {
        <a (click)="authService.logout(); closeMobileMenu()">Logout ({{ authService.email() ?? 'Wayfarer' }})</a>
      } @else {
        <a routerLink="/login" (click)="closeMobileMenu()">Login</a>
      }
    </nav>
  `,
  styleUrls: ['./header.component.scss']
})
export class HeaderComponent {
  searchService = inject(SearchService);
  authService = inject(AuthService);
  toastService = inject(ToastService);
  router = inject(Router);

  isScrolled = signal<boolean>(false);
  isMobileMenuOpen = signal<boolean>(false);
  isAccountMenuOpen = signal<boolean>(false);

  @HostListener('window:scroll', [])
  onWindowScroll() {
    this.isScrolled.set(window.scrollY > 20);
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
    this.toastService.showInfo('Official Eivar Discord & Community Guilds link active.', 'Eivar Community');
  }
}
