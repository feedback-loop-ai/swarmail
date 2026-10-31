// The Playwright fixture: `test.extend({ inbox })`. Every test execution gets
// its own inbox, named so the program under test can use the name as its
// SMTP AUTH username (decision 0004). The fixture never clears an inbox and
// never touches another one; leftover mail is the evidence a failed test needs.

import { test as base } from '@playwright/test';
import { SwarmailClient, newInboxName } from './client.js';

export { expect } from '@playwright/test';
export {
  SwarmailAmbiguous,
  SwarmailClient,
  SwarmailError,
  SwarmailHttpError,
  SwarmailTimeout,
  newInboxName,
} from './client.js';

export const test = base.extend({
  // eslint-disable-next-line no-empty-pattern
  inbox: async ({}, use) => {
    const client = new SwarmailClient();
    const name = newInboxName();
    await use({
      name,
      waitForCode: (options) => client.waitForCode(name, options),
      waitForLink: (match, options) => client.waitForLink(name, match, options),
    });
  },
});
