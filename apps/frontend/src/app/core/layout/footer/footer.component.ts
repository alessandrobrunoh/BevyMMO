import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

@Component({
  selector: 'app-footer',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <footer id="footer">
      <div class="footer-shell">
        <div class="footer-content">
          <div class="footer-grid">
            <div class="footer-brand">
              <a routerLink="/" class="footer-home-link" aria-label="Eivar Online, home">
                <img
                  src="assets/branding/eivar-online-logo-vector.svg"
                  alt="Eivar Online"
                  class="footer-vector-svg"
                  width="1185"
                  height="400"
                />
              </a>
              <p>A fantasy MMORPG where ancient words shape power, and legends are forged.</p>

              <nav class="footer-socials" aria-label="Eivar Online social channels">
                <a href="#" aria-label="Discord" class="soc-icon" title="Discord">
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M8.2 7.2a11 11 0 0 1 7.6 0 8.7 8.7 0 0 1 2.1 8.1 8.6 8.6 0 0 1-2.7 1.4l-.7-1a6.7 6.7 0 0 0 1.5-.8 10 10 0 0 1-8 0 7 7 0 0 0 1.5.8l-.7 1a8.6 8.6 0 0 1-2.7-1.4 8.7 8.7 0 0 1 2.1-8.1Z"></path>
                    <circle cx="9.4" cy="12.1" r="1"></circle>
                    <circle cx="14.6" cy="12.1" r="1"></circle>
                  </svg>
                </a>
                <a href="#" aria-label="X" class="soc-icon" title="X">
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path d="m6.7 5 10.6 14M17.7 5 6.3 19"></path>
                  </svg>
                </a>
                <a href="#" aria-label="YouTube" class="soc-icon" title="YouTube">
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <rect x="3.5" y="6.5" width="17" height="11" rx="3"></rect>
                    <path d="m10.5 9.5 4 2.5-4 2.5Z"></path>
                  </svg>
                </a>
                <a href="#" aria-label="Instagram" class="soc-icon" title="Instagram">
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <rect x="4" y="4" width="16" height="16" rx="4"></rect>
                    <circle cx="12" cy="12" r="3.5"></circle>
                    <circle cx="17.2" cy="6.9" r=".7"></circle>
                  </svg>
                </a>
              </nav>

              <img
                class="footer-brand-divider"
                src="assets/images/footer/footer-divider.webp"
                alt=""
                width="53"
                height="320"
                aria-hidden="true"
              />
            </div>

            <nav class="footer-nav" aria-label="Footer navigation">
              <div class="footer-column">
                <h2>Game</h2>
                <a routerLink="/" fragment="world">Overview</a>
                <a routerLink="/" fragment="pillars">Features</a>
                <a routerLink="/wiki/weapons/channeling-staff">Classes</a>
                <a routerLink="/wiki">World</a>
                <a routerLink="/wiki">FAQ</a>
              </div>

              <div class="footer-column">
                <h2>News</h2>
                <a routerLink="/news">Latest News</a>
                <a routerLink="/updates">Updates</a>
                <a routerLink="/news">Dev Blog</a>
                <a routerLink="/updates">Patch Notes</a>
              </div>

              <div class="footer-column">
                <h2>Wiki</h2>
                <a routerLink="/wiki">Getting Started</a>
                <a routerLink="/wiki">Guides</a>
                <a routerLink="/wiki">Lore</a>
                <a routerLink="/wiki/weapons/channeling-staff">Items</a>
                <a routerLink="/wiki">Monsters</a>
              </div>

              <div class="footer-column">
                <h2>Community</h2>
                <a href="#">Discord</a>
                <a href="#">Forums</a>
                <a href="#">Guilds</a>
                <a href="#">Events</a>
                <a href="#">Media</a>
              </div>

              <div class="footer-column">
                <h2>Support</h2>
                <a href="#">Support Center</a>
                <a href="#">Bug Reports</a>
                <a href="#">Terms of Service</a>
                <a href="#">Privacy Policy</a>
                <a href="#">Code of Conduct</a>
              </div>
            </nav>
          </div>

          <div class="footer-bottom">
            <span>© 2026 Eivar Online. All rights reserved.</span>
          </div>
        </div>
      </div>
    </footer>
  `,
  styleUrls: ['./footer.component.scss']
})
export class FooterComponent {}
