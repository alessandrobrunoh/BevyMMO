import { Injectable, inject, signal } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { API_BASE_URL } from '../../core/config/api.config';
import { AuthService } from '../../core/services/auth.service';
import {
  ItemTicket,
  MarketSummary,
  SellOffer,
  WalletResponse
} from '../../shared/models/market.model';

const SELECTED_CHARACTER_KEY = 'eivar.market.characterId';

@Injectable({
  providedIn: 'root'
})
export class MarketService {
  private http = inject(HttpClient);
  private auth = inject(AuthService);

  readonly selectedCharacterId = signal<string | null>(null);
  readonly gold = signal<number | null>(null);

  listMarkets(): Promise<MarketSummary[]> {
    return this.request<MarketSummary[]>('/public/markets');
  }

  listOffers(marketId: string): Promise<SellOffer[]> {
    return this.request<SellOffer[]>(
      `/public/markets/${encodeURIComponent(marketId)}/offers`
    );
  }

  getTicket(marketId: string, itemId: string): Promise<ItemTicket> {
    return this.request<ItemTicket>(
      `/public/markets/${encodeURIComponent(marketId)}/items/${encodeURIComponent(itemId)}`
    );
  }

  getWallet(characterId: string): Promise<WalletResponse> {
    return this.request<WalletResponse>(`/characters/${encodeURIComponent(characterId)}/wallet`);
  }

  /**
   * Picks a character (stored, first roster entry, or explicit) and loads
   * its Gold. No-op when anonymous.
   */
  async syncWallet(explicitId?: string): Promise<void> {
    if (!this.auth.isLoggedIn()) {
      this.gold.set(null);
      this.selectedCharacterId.set(null);
      return;
    }
    const characters = this.auth.profile()?.characters ?? [];
    const stored = localStorage.getItem(SELECTED_CHARACTER_KEY);
    const chosen =
      explicitId ??
      characters.find(c => c.character_id === stored)?.character_id ??
      characters[0]?.character_id ??
      null;
    this.selectedCharacterId.set(chosen);
    if (!chosen) {
      this.gold.set(0);
      return;
    }
    localStorage.setItem(SELECTED_CHARACTER_KEY, chosen);
    try {
      const wallet = await this.getWallet(chosen);
      this.gold.set(wallet.gold);
    } catch {
      this.gold.set(0);
    }
  }

  private async request<T>(path: string): Promise<T> {
    const url = `${API_BASE_URL}${path}`;
    try {
      return await firstValueFrom(this.http.get<T>(url, { withCredentials: true }));
    } catch (err) {
      throw new Error(describeGatewayError(err, path));
    }
  }
}

/** HTML 200 from `ng serve` (no `/v1` proxy) looks like a failed JSON parse. */
export function describeGatewayError(err: unknown, path: string): string {
  if (!(err instanceof HttpErrorResponse)) {
    return err instanceof Error ? err.message : 'Could not reach the market.';
  }
  if (looksLikeSpaFallback(err)) {
    return 'The gateway is not reachable at /v1. Start bevymmo_gateway on :8081 and use `npm start` (proxies /v1).';
  }
  const body = err.error as { error?: unknown } | string | null;
  if (typeof body === 'object' && body && typeof body.error === 'string') {
    return body.error;
  }
  return `Request to ${path} failed (${err.status}).`;
}

function looksLikeSpaFallback(err: HttpErrorResponse): boolean {
  if (err.status === 0) {
    return true;
  }
  if (typeof err.error === 'string' && /<!doctype html/i.test(err.error)) {
    return true;
  }
  const nested = (err.error as { error?: unknown; text?: string } | null)?.error;
  if (nested instanceof SyntaxError) {
    return true;
  }
  if (err.status === 200) {
    return true;
  }
  return false;
}
