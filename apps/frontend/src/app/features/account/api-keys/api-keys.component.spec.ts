import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { ApiKeysComponent } from './api-keys.component';
import { ApiKeyListItem, ApiKeyService, CreatedApiKey } from '../../../core/services/api-key.service';
import { ToastService } from '../../../core/services/toast.service';

describe('ApiKeysComponent', () => {
  let fixture: ComponentFixture<ApiKeysComponent>;
  let list: ReturnType<typeof vi.fn>;
  let create: ReturnType<typeof vi.fn>;
  let revoke: ReturnType<typeof vi.fn>;

  const listed: ApiKeyListItem = {
    id: '11111111-1111-4111-8111-111111111111',
    name: 'discord-bot',
    prefix: 'eiv_a1b2c3d4',
    created_at: 1_768_000_000_000_000,
    last_used_at: null
  };

  const created: CreatedApiKey = {
    ...listed,
    key: 'eiv_a1b2c3d4' + 'ab'.repeat(28)
  };

  beforeEach(async () => {
    list = vi.fn().mockResolvedValue([]);
    create = vi.fn().mockResolvedValue(created);
    revoke = vi.fn().mockResolvedValue(undefined);

    await TestBed.configureTestingModule({
      imports: [ApiKeysComponent],
      providers: [
        provideRouter([]),
        { provide: ApiKeyService, useValue: { list, create, revoke } },
        {
          provide: ToastService,
          useValue: { showSuccess: vi.fn(), showWarning: vi.fn(), showInfo: vi.fn() }
        }
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(ApiKeysComponent);
  });

  async function render(): Promise<void> {
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  it('shows an empty state when the account has no keys', async () => {
    await render();

    const empty = fixture.nativeElement.querySelector('[data-testid="empty-state"]');
    expect(empty).toBeTruthy();
    expect(empty.textContent).toContain('No API keys yet');
    expect(fixture.nativeElement.querySelector('.key-row')).toBeNull();
  });

  it('lists the prefix and never the full secret', async () => {
    list.mockResolvedValue([listed]);
    await render();

    const row = fixture.nativeElement.querySelector('.key-row');
    expect(row.textContent).toContain('discord-bot');
    expect(row.textContent).toContain('eiv_a1b2c3d4');
    expect(row.textContent).not.toContain(created.key);
    expect(fixture.nativeElement.querySelector('[data-testid="revealed-secret"]')).toBeNull();
  });

  it('reveals the secret once after create and keeps it out of the list', async () => {
    list.mockResolvedValueOnce([]).mockResolvedValue([listed]);
    await render();

    fixture.componentInstance.createName.set('discord-bot');
    await fixture.componentInstance.submitCreate();
    fixture.detectChanges();

    const secret = fixture.nativeElement.querySelector('[data-testid="revealed-secret"]');
    expect(secret.textContent).toContain(created.key);

    const row = fixture.nativeElement.querySelector('.key-row');
    expect(row.textContent).toContain('eiv_a1b2c3d4');
    expect(row.textContent).not.toContain(created.key);
  });

  it('returns to the empty state after the last key is revoked', async () => {
    list.mockResolvedValueOnce([listed]).mockResolvedValue([]);
    await render();

    fixture.componentInstance.askRevoke(listed);
    await fixture.componentInstance.confirmRevoke();
    fixture.detectChanges();

    expect(revoke).toHaveBeenCalledWith(listed.id);
    expect(fixture.nativeElement.querySelector('[data-testid="empty-state"]')).toBeTruthy();
  });
});
