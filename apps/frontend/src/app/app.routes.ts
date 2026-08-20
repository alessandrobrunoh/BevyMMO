import { Routes } from '@angular/router';
import { authGuard } from './core/guards/auth.guard';

export const routes: Routes = [
  {
    path: '',
    loadComponent: () => import('./features/home/home.component').then(m => m.HomeComponent),
    title: 'Eivar Online — An Evolving Fantasy MMORPG'
  },
  {
    path: 'news',
    loadComponent: () => import('./features/news/news-list/news-list.component').then(m => m.NewsListComponent),
    title: 'News & Chronicles — Eivar Online'
  },
  {
    path: 'news/:slug',
    loadComponent: () => import('./features/news/news-detail/news-detail.component').then(m => m.NewsDetailComponent),
    title: 'Chronicle — Eivar Online'
  },
  {
    path: 'updates',
    loadComponent: () => import('./features/updates/updates.component').then(m => m.UpdatesComponent),
    title: 'Development & Patch Notes — Eivar Online'
  },
  {
    path: 'wiki',
    loadComponent: () => import('./features/wiki/wiki-landing/wiki-landing.component').then(m => m.WikiLandingComponent),
    title: 'The Eivar Archives — Official Codex'
  },
  {
    path: 'wiki/:category',
    loadComponent: () => import('./features/wiki/wiki-detail/wiki-detail.component').then(m => m.WikiDetailComponent),
    title: 'Codex Section — Eivar Online'
  },
  {
    path: 'wiki/:category/:slug',
    loadComponent: () => import('./features/wiki/wiki-detail/wiki-detail.component').then(m => m.WikiDetailComponent),
    title: 'Codex Record — Eivar Online'
  },
  {
    path: 'store',
    loadComponent: () => import('./features/store/store.component').then(m => m.StoreComponent),
    title: 'Supporter Store — Eivar Online'
  },
  {
    path: 'market',
    loadComponent: () =>
      import('./features/market/market-list.component').then(m => m.MarketListComponent),
    title: 'Player Markets — Eivar Online'
  },
  {
    path: 'market/:marketId',
    loadComponent: () =>
      import('./features/market/market-browse.component').then(m => m.MarketBrowseComponent),
    title: 'Market Hall — Eivar Online'
  },
  {
    path: 'market/:marketId/:itemId',
    loadComponent: () =>
      import('./features/market/market-ticket.component').then(m => m.MarketTicketComponent),
    title: 'Item Ticket — Eivar Online'
  },
  {
    path: 'login',
    loadComponent: () => import('./features/auth/login.component').then(m => m.LoginComponent),
    title: 'Alpha Login — Eivar Online'
  },
  {
    path: 'profile',
    loadComponent: () => import('./features/profile/profile.component').then(m => m.ProfileComponent),
    canActivate: [authGuard],
    title: 'Your Profile — Eivar Online'
  },
  {
    path: '**',
    redirectTo: ''
  }
];
