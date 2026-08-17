import { Injectable, inject, signal } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { API_BASE_URL } from '../config/api.config';

export interface CharacterSummary {
  character_id: number;
  display_name: string;
  online: boolean;
}

export interface AccountProfile {
  account_id: number;
  characters: CharacterSummary[];
}

/**
 * Real authentication against `apps/gateway`'s `/auth/*` and `/profile`.
 * Replaces `AuthMockService`.
 *
 * The gateway holds the actual session (a live SpacetimeDB connection behind
 * an `HttpOnly` cookie — see `apps/gateway/src/stdb/session.rs`); this
 * service only mirrors what the gateway reports into signals the UI reads.
 * `withCredentials: true` on every call is required, not optional: without
 * it the browser never sends or stores the session cookie cross-origin.
 */
@Injectable({
  providedIn: 'root'
})
export class AuthService {
  private http = inject(HttpClient);

  readonly isLoggedIn = signal<boolean>(false);
  readonly profile = signal<AccountProfile | null>(null);
  /**
   * The email just used to log in or register, kept client-side only.
   * `/profile` does not return it — the gateway never reads `Account` back
   * from SpacetimeDB (that table is deliberately private; see
   * `tables::Account`'s doc comment), so a session restored purely from the
   * cookie (`restoreSession`, e.g. on page reload) has no email to show.
   */
  readonly email = signal<string | null>(null);

  async register(email: string, password: string): Promise<void> {
    const profile = await this.request<AccountProfile>('POST', '/auth/register', { email, password });
    this.email.set(email);
    this.setAuthenticated(profile);
  }

  async login(email: string, password: string): Promise<void> {
    const profile = await this.request<AccountProfile>('POST', '/auth/login', { email, password });
    this.email.set(email);
    this.setAuthenticated(profile);
  }

  async logout(): Promise<void> {
    try {
      await this.request('POST', '/auth/logout');
    } finally {
      this.clearAuthenticated();
    }
  }

  /**
   * Checks whether a session cookie from a previous visit is still valid.
   * Call once at app startup (see `app.config.ts`); a 401 here is normal
   * (never logged in, or the gateway's 30-minute idle timeout expired the
   * underlying SpacetimeDB connection — see `SessionStore`), not an error to
   * surface to the user.
   */
  async restoreSession(): Promise<void> {
    try {
      const profile = await this.request<AccountProfile>('GET', '/profile');
      this.setAuthenticated(profile);
    } catch {
      this.clearAuthenticated();
    }
  }

  private setAuthenticated(profile: AccountProfile) {
    this.isLoggedIn.set(true);
    this.profile.set(profile);
  }

  private clearAuthenticated() {
    this.isLoggedIn.set(false);
    this.profile.set(null);
    this.email.set(null);
  }

  /** Turns the gateway's `{ error: string }` body into a thrown `Error` with that message. */
  private async request<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<T> {
    const url = `${API_BASE_URL}${path}`;
    const options = { withCredentials: true };
    try {
      const response$ =
        method === 'GET'
          ? this.http.get<T>(url, options)
          : this.http.post<T>(url, body ?? {}, options);
      return await firstValueFrom(response$);
    } catch (err) {
      if (err instanceof HttpErrorResponse) {
        const message = (err.error as { error?: string } | null)?.error;
        throw new Error(message ?? `Request to ${path} failed (${err.status}).`);
      }
      throw err;
    }
  }
}
