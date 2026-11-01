import { defineConfig } from '@playwright/test';

// No browser projects: the fixture is a REST client, and these examples never
// open a page. One worker, no retries — a red run must be a real failure.
export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.ts',
  workers: 1,
  retries: 0,
  reporter: 'list',
});
