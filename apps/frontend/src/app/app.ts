import { Component, inject, signal } from '@angular/core';
import { NavigationEnd, Router, RouterOutlet } from '@angular/router';
import { filter } from 'rxjs';
import { HeaderComponent } from './core/layout/header/header.component';
import { FooterComponent } from './core/layout/footer/footer.component';
import { SearchOverlayComponent } from './shared/ui/search-overlay/search-overlay.component';
import { ToastContainerComponent } from './core/layout/toast/toast.component';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    RouterOutlet,
    HeaderComponent,
    FooterComponent,
    SearchOverlayComponent,
    ToastContainerComponent
  ],
  templateUrl: './app.html',
  styleUrl: './app.scss'
})
export class App {
  private router = inject(Router);

  readonly isAuthRoute = signal(this.isBareRoute(this.router.url));

  constructor() {
    this.router.events
      .pipe(filter((event): event is NavigationEnd => event instanceof NavigationEnd))
      .subscribe(event => this.isAuthRoute.set(this.isBareRoute(event.urlAfterRedirects)));
  }

  private isBareRoute(url: string): boolean {
    return url.startsWith('/login') || url.startsWith('/register');
  }
}
