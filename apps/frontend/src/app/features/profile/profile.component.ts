import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { PageHeaderComponent } from '../../shared/ui/page-header/page-header.component';
import { AuthService } from '../../core/services/auth.service';

/**
 * `/profile` — the account's own characters. Reachable only when
 * authenticated (`authGuard`); the data itself comes from `AuthService`,
 * already scoped to the caller by the gateway (`GET /profile` resolves the
 * account from the session cookie server-side, never from anything the
 * client sends).
 */
@Component({
  selector: 'app-profile',
  standalone: true,
  imports: [CommonModule, PageHeaderComponent],
  template: `
    <app-page-header title="Your Profile" [breadcrumbs]="[{ label: 'Profile' }]" />

    <section class="container profile-section">
      @if (authService.email(); as email) {
        <p class="profile-email">Signed in as {{ email }}</p>
      }

      <h2 class="profile-heading">Characters</h2>

      @if (characters().length > 0) {
        <ul class="character-list">
          @for (character of characters(); track character.character_id) {
            <li class="character-row">
              <span class="character-name">{{ character.display_name }}</span>
              @if (character.online) {
                <span class="character-status online">Online</span>
              } @else {
                <span class="character-status">Offline</span>
              }
            </li>
          }
        </ul>
      } @else {
        <p class="profile-empty">No characters yet — create one from the game client.</p>
      }
    </section>
  `,
  styles: [
    `
      .profile-section {
        padding: 2rem 0 4rem;
        max-width: 640px;
      }
      .profile-email {
        opacity: 0.7;
        margin-bottom: 1.5rem;
      }
      .profile-heading {
        margin-bottom: 1rem;
      }
      .character-list {
        list-style: none;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }
      .character-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem 1rem;
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 4px;
      }
      .character-status {
        opacity: 0.6;
        font-size: 0.85rem;
      }
      .character-status.online {
        color: #3ccbff;
        opacity: 1;
      }
      .profile-empty {
        opacity: 0.6;
      }
    `
  ]
})
export class ProfileComponent {
  authService = inject(AuthService);

  characters() {
    return this.authService.profile()?.characters ?? [];
  }
}
