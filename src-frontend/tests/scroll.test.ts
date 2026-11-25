import { describe, expect, it } from 'vitest';
import { isNearBottom } from '../src/lib/chat/scroll';

describe('chat follow-scroll threshold', () => {
  it('follows the stream at the bottom or within the threshold', () => {
    expect(isNearBottom({ scrollHeight: 1000, clientHeight: 400, scrollTop: 600 })).toBe(true);
    expect(isNearBottom({ scrollHeight: 1000, clientHeight: 400, scrollTop: 552 })).toBe(true);
  });
  it('does not follow when reading previous messages', () => {
    expect(isNearBottom({ scrollHeight: 1000, clientHeight: 400, scrollTop: 551 })).toBe(false);
    expect(isNearBottom({ scrollHeight: 1000, clientHeight: 400, scrollTop: 0 })).toBe(false);
  });
  it('handles short history and fractional browser measurements', () => {
    expect(isNearBottom({ scrollHeight: 200, clientHeight: 400, scrollTop: 0 })).toBe(true);
    expect(isNearBottom({ scrollHeight: 1000, clientHeight: 400, scrollTop: 599.5 })).toBe(true);
  });
});
