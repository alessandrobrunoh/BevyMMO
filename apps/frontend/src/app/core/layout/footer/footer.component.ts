import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { ToastService } from '../../services/toast.service';

@Component({
  selector: 'app-footer',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <footer class="global-footer">
      <div class="footer-silhouette-bg"></div>

      <div class="container footer-content">
        <div class="footer-top">
          <!-- Brand Column -->
          <div class="footer-brand-col">
            <div class="footer-logo">
              <svg viewBox="0 0 100 160" class="footer-rune-svg" fill="none">
                <path d="M50 14 L54 30 L50 26 L46 30 Z" fill="#3ccbff" />
                <line x1="50" y1="14" x2="50" y2="40" stroke="#3ccbff" stroke-width="3" />
                <path d="M50 35 L68 56 L50 78 L32 56 Z" stroke="#3ccbff" stroke-width="3.5" fill="none" />
                <line x1="24" y1="56" x2="76" y2="56" stroke="#3ccbff" stroke-width="3" />
                <circle cx="50" cy="80" r="4" fill="#e6cb86" />
                <path d="M50 82 L68 104 L50 126 L32 104 Z" stroke="#3ccbff" stroke-width="3.5" fill="none" />
                <line x1="24" y1="104" x2="76" y2="104" stroke="#3ccbff" stroke-width="3" />
                <line x1="50" y1="120" x2="50" y2="146" stroke="#3ccbff" stroke-width="3" />
              </svg>
              <div class="brand-text">
                <h3 class="footer-brand-title">EIVAR</h3>
                <span class="footer-brand-sub">ONLINE</span>
              </div>
            </div>
            <p class="footer-desc">
              An evolving fantasy online world of ancient stone monoliths, floating lands, customizable weapons, and runic magic.
            </p>
          </div>

          <!-- Links Columns -->
          <div class="footer-nav-groups">
            <div class="nav-col">
              <h4 class="col-title">Game</h4>
              <ul class="col-list">
                <li><a routerLink="/">Overview & Lore</a></li>
                <li><a routerLink="/news">Latest News</a></li>
                <li><a routerLink="/updates">Patch Notes & Builds</a></li>
                <li><a routerLink="/wiki">The Eivar Archives</a></li>
                <li><a routerLink="/store">Supporter Store</a></li>
              </ul>
            </div>

            <div class="nav-col">
              <h4 class="col-title">Knowledge</h4>
              <ul class="col-list">
                <li><a routerLink="/wiki/weapons/channeling-staff">Channeling Staff</a></li>
                <li><a routerLink="/wiki/essences/essences-overview">Primal Essences</a></li>
                <li><a routerLink="/wiki/ancient-words/ancient-word-echo">Ancient Word: Echo</a></li>
                <li><a routerLink="/wiki">All Categories</a></li>
              </ul>
            </div>

            <div class="nav-col">
              <h4 class="col-title">Community</h4>
              <ul class="col-list">
                <li><a (click)="onSocialClick('Discord')">Official Discord</a></li>
                <li><a (click)="onSocialClick('Development Forum')">Development Forum</a></li>
                <li><a (click)="onSocialClick('Alpha Playtesting')">Alpha Playtesting</a></li>
                <li><a (click)="onSocialClick('Community Guilds')">Community Guilds</a></li>
              </ul>
            </div>

            <div class="nav-col">
              <h4 class="col-title">Legal</h4>
              <ul class="col-list">
                <li><a (click)="onLegalClick('Terms of Service')">Terms of Service</a></li>
                <li><a (click)="onLegalClick('Privacy Policy')">Privacy Policy</a></li>
                <li><a (click)="onLegalClick('Cookie Settings')">Cookie Settings</a></li>
                <li><a (click)="onLegalClick('Code of Conduct')">Code of Conduct</a></li>
              </ul>
            </div>
          </div>
        </div>

        <div class="footer-divider"></div>

        <div class="footer-bottom">
          <div class="disclaimer-box">
            <span class="rune-symbol">ᛟ</span>
            <p class="disclaimer-text">
              <strong>DEVELOPMENT NOTICE:</strong> Eivar Online is currently in active pre-alpha development. All gameplay systems, lore descriptions, weapon mechanics, and visual assets shown across this portal represent prototype work in progress.
            </p>
          </div>
          <p class="copyright">
            © 2026 Eivar Online Project. Built with love for the online fantasy genre.
          </p>
        </div>
      </div>
    </footer>
  `,
  styleUrls: ['./footer.component.scss']
})
export class FooterComponent {
  toastService = inject(ToastService);

  onSocialClick(channel: string) {
    this.toastService.showInfo(`Connecting to Eivar ${channel} community...`, 'Community Hub');
  }

  onLegalClick(doc: string) {
    this.toastService.showInfo(`${doc} displayed for prototype purposes.`, 'Legal Notice');
  }
}
