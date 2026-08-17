import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

@Component({
  selector: 'app-footer',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <footer id="footer">
      <div class="container">
        <div class="footer-grid">
          <!-- Brand Vector Logo -->
          <div class="footer-brand">
            <img
              src="assets/branding/eivar-online-logo-vector.svg"
              alt="Eivar Online"
              class="footer-vector-svg"
            />
            <p>
              A fantasy MMORPG where ancient words shape power, and legends are forged.
            </p>

            <!-- Social Links using Google Fonts icons -->
            <div class="footer-socials">
              <a href="#" aria-label="Discord" class="soc-icon" title="Discord">
                <span class="material-symbols-outlined">forum</span>
              </a>
              <a href="#" aria-label="Community" class="soc-icon" title="Community">
                <span class="material-symbols-outlined">groups</span>
              </a>
              <a href="#" aria-label="Videos" class="soc-icon" title="Media & Video">
                <span class="material-symbols-outlined">smart_display</span>
              </a>
              <a href="#" aria-label="Lore" class="soc-icon" title="Archives">
                <span class="material-symbols-outlined">auto_stories</span>
              </a>
            </div>
          </div>

          <!-- Column 1: Game -->
          <div class="footer-column">
            <h4>Game</h4>
            <a routerLink="/" fragment="world">Overview</a>
            <a routerLink="/" fragment="pillars">Features</a>
            <a routerLink="/wiki/weapons/channeling-staff">Classes</a>
            <a routerLink="/wiki">World</a>
            <a routerLink="/wiki">FAQ</a>
          </div>

          <!-- Column 2: News -->
          <div class="footer-column">
            <h4>News</h4>
            <a routerLink="/news">Latest News</a>
            <a routerLink="/updates">Updates</a>
            <a routerLink="/news">Dev Blog</a>
            <a routerLink="/updates">Patch Notes</a>
          </div>

          <!-- Column 3: Wiki -->
          <div class="footer-column">
            <h4>Wiki</h4>
            <a routerLink="/wiki">Getting Started</a>
            <a routerLink="/wiki">Guides</a>
            <a routerLink="/wiki">Lore</a>
            <a routerLink="/wiki/weapons/channeling-staff">Items</a>
            <a routerLink="/wiki">Monsters</a>
          </div>

          <!-- Column 4: Community -->
          <div class="footer-column">
            <h4>Community</h4>
            <a href="#">Discord</a>
            <a href="#">Forums</a>
            <a href="#">Guilds</a>
            <a href="#">Events</a>
            <a href="#">Media</a>
          </div>

          <!-- Column 5: Support -->
          <div class="footer-column">
            <h4>Support</h4>
            <a href="#">Support Center</a>
            <a href="#">Bug Reports</a>
            <a href="#">Terms of Service</a>
            <a href="#">Privacy Policy</a>
            <a href="#">Code of Conduct</a>
          </div>
        </div>

        <div class="footer-bottom">
          <span>© 2026 Eivar Online. All rights reserved.</span>
        </div>
      </div>
    </footer>
  `,
  styleUrls: ['./footer.component.scss']
})
export class FooterComponent {}
