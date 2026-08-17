import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
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
export class App {}
