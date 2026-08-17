import { Component, inject, HostListener, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router } from '@angular/router';
import { SearchService } from '../../services/search.service';
import { AuthMockService } from '../../services/auth-mock.service';
import { ToastService } from '../../services/toast.service';
import { AccountMenuComponent } from '../../../shared/ui/account-menu/account-menu.component';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [CommonModule, RouterModule, AccountMenuComponent],
  template: `
    <header class="navbar" [class.scrolled]="isScrolled()">
      <div class="nav-inner">
        <!-- Official Vector Brand Logo -->
        <a routerLink="/" class="brand" (click)="closeMobileMenu()">
          <img
            src="assets/branding/eivar-online-logo-vector.svg"
            alt="Eivar Online"
            class="brand-vector-svg"
          />
        </a>

        <!-- Desktop Navigation Links -->
        <nav class="desktop-nav">
          <a routerLink="/" routerLinkActive="active" [routerLinkActiveOptions]="{exact: true}" class="nav-link">Game</a>
          <span class="nav-sep">◈</span>
          <a routerLink="/news" routerLinkActive="active" class="nav-link">News</a>
          <span class="nav-sep">◈</span>
          <a routerLink="/updates" routerLinkActive="active" class="nav-link">Updates</a>
          <span class="nav-sep">◈</span>
          <a routerLink="/wiki" routerLinkActive="active" class="nav-link">Wiki</a>
          <span class="nav-sep">◈</span>
          <a routerLink="/store" routerLinkActive="active" class="nav-link">Store</a>
        </nav>

        <!-- Right Nav Actions with Google Fonts Icons -->
        <div class="nav-actions">
          <button class="nav-action search-btn" (click)="searchService.open()" title="Search (Cmd+K)">
            <span class="material-symbols-outlined nav-ico">search</span>
            <span>Search</span>
          </button>

          <button class="nav-action" (click)="onCommunityClick()">
            <span class="material-symbols-outlined nav-ico">forum</span>
            <span>Community</span>
          </button>

          @if (authService.isLoggedIn()) {
            <div class="account-wrapper">
              <button class="nav-action account-btn" (click)="toggleAccountMenu()">
                <span class="material-symbols-outlined nav-ico">person</span>
                <span>{{ authService.currentUser()?.name }}</span>
                <span class="material-symbols-outlined arrow-ico">expand_more</span>
              </button>
              @if (isAccountMenuOpen()) {
                <app-account-menu (closeMenu)="isAccountMenuOpen.set(false)"></app-account-menu>
              }
            </div>
          } @else {
            <a routerLink="/login" class="nav-action">
              <span class="material-symbols-outlined nav-ico">person</span>
              <span>Login</span>
            </a>
          }

          <a routerLink="/" fragment="world">
            <button class="eivar-button">
              Discover Eivar
            </button>
          </a>

          <button
            class="mobile-menu-button"
            (click)="toggleMobileMenu()"
            aria-label="Open menu"
          >
            <span class="material-symbols-outlined">menu</span>
          </button>
        </div>
      </div>
    </header>

    <!-- Mobile Drawer -->
    <nav class="mobile-menu" [class.open]="isMobileMenuOpen()">
      <a routerLink="/" (click)="closeMobileMenu()">Game</a>
      <a routerLink="/news" (click)="closeMobileMenu()">News</a>
      <a routerLink="/updates" (click)="closeMobileMenu()">Updates</a>
      <a routerLink="/wiki" (click)="closeMobileMenu()">Wiki</a>
      <a routerLink="/store" (click)="closeMobileMenu()">Store</a>
      <a (click)="onCommunityClick(); closeMobileMenu()">Community</a>
      @if (authService.isLoggedIn()) {
        <a (click)="authService.logoutMock(); closeMobileMenu()">Logout ({{ authService.currentUser()?.name }})</a>
      } @else {
        <a routerLink="/login" (click)="closeMobileMenu()">Login</a>
      }
    </nav>
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
    this.isScrolled.set(window.scrollY > 30);
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
