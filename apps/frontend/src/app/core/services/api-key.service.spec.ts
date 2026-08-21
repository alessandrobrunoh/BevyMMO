import { formatUnixMicros } from './api-key.service';

describe('formatUnixMicros', () => {
  it('labels a missing timestamp as Never', () => {
    expect(formatUnixMicros(null)).toBe('Never');
    expect(formatUnixMicros(undefined)).toBe('Never');
  });

  it('converts unix microseconds into a Date-based locale string', () => {
    const micros = Date.UTC(2026, 0, 15, 12, 0, 0) * 1000;
    const formatted = formatUnixMicros(micros);
    expect(formatted).toContain('2026');
  });
});
