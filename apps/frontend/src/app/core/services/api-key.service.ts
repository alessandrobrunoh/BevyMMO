import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { API_BASE_URL } from '../config/api.config';

export interface ApiKeyListItem {
  id: string;
  name: string;
  prefix: string;
  created_at: number;
  last_used_at: number | null;
}

export interface CreatedApiKey extends ApiKeyListItem {
  /** Plaintext secret. Present only on the create response. */
  key: string;
}

/**
 * Cookie-authenticated CRUD against `/v1/api-keys`.
 * The gateway never returns the secret after create; this service does not cache it.
 */
@Injectable({
  providedIn: 'root'
})
export class ApiKeyService {
  private http = inject(HttpClient);

  list(): Promise<ApiKeyListItem[]> {
    return this.request<ApiKeyListItem[]>('GET', '/api-keys');
  }

  create(name: string): Promise<CreatedApiKey> {
    return this.request<CreatedApiKey>('POST', '/api-keys', { name });
  }

  async revoke(id: string): Promise<void> {
    await this.request<void>('DELETE', `/api-keys/${id}`);
  }

  private async request<T>(method: 'GET' | 'POST' | 'DELETE', path: string, body?: unknown): Promise<T> {
    const url = `${API_BASE_URL}${path}`;
    const options = { withCredentials: true };
    try {
      if (method === 'GET') {
        return await firstValueFrom(this.http.get<T>(url, options));
      }
      if (method === 'POST') {
        return await firstValueFrom(this.http.post<T>(url, body ?? {}, options));
      }
      await firstValueFrom(this.http.delete(url, options));
      return undefined as T;
    } catch (err) {
      if (err instanceof HttpErrorResponse) {
        const message = (err.error as { error?: string } | null)?.error;
        throw new Error(message ?? `Request to ${path} failed (${err.status}).`);
      }
      throw err;
    }
  }
}

/** Unix microseconds from the gateway → locale string. */
export function formatUnixMicros(micros: number | null | undefined): string {
  if (micros == null) {
    return 'Never';
  }
  return new Date(Math.floor(micros / 1000)).toLocaleString();
}
