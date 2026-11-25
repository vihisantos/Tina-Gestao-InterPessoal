import '@testing-library/svelte/vitest';
import { vi } from 'vitest';

Object.defineProperty(globalThis.navigator, 'clipboard', {
  configurable: true,
  value: {
    writeText: vi.fn(async () => undefined)
  }
});

if (!('ResizeObserver' in globalThis)) {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  Object.defineProperty(globalThis, 'ResizeObserver', {
    configurable: true,
    value: ResizeObserverStub
  });
}

if (!globalThis.window.matchMedia) {
  Object.defineProperty(globalThis.window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener() {},
      removeListener() {},
      addEventListener() {},
      removeEventListener() {},
      dispatchEvent() {
        return false;
      }
    })
  });
}
