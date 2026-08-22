import { Component, inject, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { PageHeaderComponent } from '../../../shared/ui/page-header/page-header.component';
import { EivarButtonComponent } from '../../../shared/ui/button/button.component';
import { ModalComponent } from '../../../shared/ui/modal/modal.component';
import { ToastService } from '../../../core/services/toast.service';
import {
  ApiKeyListItem,
  ApiKeyService,
  CreatedApiKey,
  formatUnixMicros
} from '../../../core/services/api-key.service';

@Component({
  selector: 'app-api-keys',
  standalone: true,
  imports: [CommonModule, FormsModule, PageHeaderComponent, EivarButtonComponent, ModalComponent],
  templateUrl: './api-keys.component.html',
  styleUrl: './api-keys.component.scss'
})
export class ApiKeysComponent implements OnInit {
  private apiKeysApi = inject(ApiKeyService);
  private toast = inject(ToastService);

  readonly keys = signal<ApiKeyListItem[]>([]);
  readonly loading = signal(true);
  readonly errorMessage = signal<string | null>(null);

  readonly createOpen = signal(false);
  readonly createName = signal('');
  readonly creating = signal(false);
  readonly createError = signal<string | null>(null);

  readonly revealed = signal<CreatedApiKey | null>(null);
  readonly copied = signal(false);

  readonly pendingRevoke = signal<ApiKeyListItem | null>(null);
  readonly revoking = signal(false);

  formatDate = formatUnixMicros;

  curlExample(key = 'eiv_…'): string {
    const origin = typeof window === 'undefined' ? '' : window.location.origin;
    return `curl -H "Authorization: Bearer ${key}" ${origin}/v1/profile`;
  }

  async ngOnInit(): Promise<void> {
    await this.reload();
  }

  async reload(): Promise<void> {
    this.loading.set(true);
    this.errorMessage.set(null);
    try {
      const keys = await this.apiKeysApi.list();
      this.keys.set(keys);
    } catch (err) {
      this.errorMessage.set(err instanceof Error ? err.message : 'Could not load API keys.');
    } finally {
      this.loading.set(false);
    }
  }

  openCreate(): void {
    this.createName.set('');
    this.createError.set(null);
    this.createOpen.set(true);
  }

  closeCreate(): void {
    if (!this.creating()) {
      this.createOpen.set(false);
    }
  }

  async submitCreate(): Promise<void> {
    const name = this.createName().trim();
    if (!name) {
      this.createError.set('Give this key a name so you can tell it apart later.');
      return;
    }
    this.creating.set(true);
    this.createError.set(null);
    try {
      const created = await this.apiKeysApi.create(name);
      this.createOpen.set(false);
      this.revealed.set(created);
      this.copied.set(false);
      await this.reload();
    } catch (err) {
      this.createError.set(err instanceof Error ? err.message : 'Could not create the key.');
    } finally {
      this.creating.set(false);
    }
  }

  closeReveal(): void {
    this.revealed.set(null);
    this.copied.set(false);
  }

  async copySecret(): Promise<void> {
    const secret = this.revealed()?.key;
    if (!secret) {
      return;
    }
    try {
      await navigator.clipboard.writeText(secret);
      this.copied.set(true);
      this.toast.showSuccess('The secret is on your clipboard. Store it now — it will not be shown again.', 'Copied');
    } catch {
      this.toast.showWarning('Could not copy automatically. Select the key and copy it yourself.', 'Clipboard');
    }
  }

  askRevoke(key: ApiKeyListItem): void {
    this.pendingRevoke.set(key);
  }

  closeRevoke(): void {
    if (!this.revoking()) {
      this.pendingRevoke.set(null);
    }
  }

  async confirmRevoke(): Promise<void> {
    const key = this.pendingRevoke();
    if (!key) {
      return;
    }
    this.revoking.set(true);
    try {
      await this.apiKeysApi.revoke(key.id);
      this.pendingRevoke.set(null);
      this.toast.showSuccess(`Revoked ${key.name}. Scripts using that secret will get 401.`, 'API key revoked');
      await this.reload();
    } catch (err) {
      this.toast.showWarning(err instanceof Error ? err.message : 'Could not revoke the key.', 'Revoke failed');
    } finally {
      this.revoking.set(false);
    }
  }
}
