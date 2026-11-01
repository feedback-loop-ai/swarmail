// The six example tests, the same six as clients/pytest/examples/test_example.py.
// They run against a built `swarmail serve` (SWARMAIL_URL, default
// http://127.0.0.1:8025). Seeding goes through REST `POST .../seed` — the same
// parse and extraction path as SMTP (decision 0010); Node has no stdlib SMTP
// client, so the SMTP AUTH path is exercised by the pytest example and the
// Rust suites.

import { test, expect, SwarmailAmbiguous, SwarmailTimeout } from '../index.js';

const base = (process.env.SWARMAIL_URL ?? 'http://127.0.0.1:8025').replace(/\/+$/, '');

async function seed(inbox: string, mail: { subject?: string; text: string }): Promise<void> {
  const res = await fetch(`${base}/api/v1/inboxes/${encodeURIComponent(inbox)}/seed`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ to: `${inbox}@example.test`, ...mail }),
  });
  expect(res.status, await res.clone().text()).toBe(200);
}

// Gate on the server's own readiness gauge, never on a sleep (decision 0020):
// the wait is parked once its inbox's row reads 1. The poll has a deadline.
async function waitParked(inbox: string): Promise<void> {
  const row = `swarmail_awaits_active{inbox="${inbox}"} 1`;
  const deadline = Date.now() + 10_000;
  for (;;) {
    const metrics = await (await fetch(`${base}/metrics`)).text();
    if (metrics.split('\n').includes(row)) return;
    if (Date.now() > deadline) throw new Error(`the gauge never showed ${row}`);
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

test('1. one mail with one code and one link: both waits return them', async ({ inbox }) => {
  await seed(inbox.name, {
    subject: 'Verify your account',
    text: 'Your code is 482913. Confirm here: https://app.example.test/verify?token=abc',
  });
  const code = await inbox.waitForCode();
  expect(code.code).toBe('482913');
  expect(code.email.inbox).toBe(inbox.name);
  const link = await inbox.waitForLink();
  expect(link.link).toBe('https://app.example.test/verify?token=abc');
  expect(link.email.id).toBe(code.email.id);
});

test('2. two distinct codes in one mail: waitForCode is ambiguous', async ({ inbox }) => {
  await seed(inbox.name, { text: 'Codes 111111 and 222222 were issued.' });
  const error = await inbox.waitForCode().then(() => null, (e: unknown) => e);
  expect(error).toBeInstanceOf(SwarmailAmbiguous);
  expect((error as SwarmailAmbiguous).candidates.sort()).toEqual(['111111', '222222']);
});

test('3. two matching links across two mails: waitForLink is ambiguous, a predicate selects', async ({ inbox }) => {
  await seed(inbox.name, { text: 'First: https://app.example.test/a' });
  await seed(inbox.name, { text: 'Second: https://app.example.test/b' });
  const error = await inbox.waitForLink().then(() => null, (e: unknown) => e);
  expect(error).toBeInstanceOf(SwarmailAmbiguous);
  expect((error as SwarmailAmbiguous).candidates.sort()).toEqual([
    'https://app.example.test/a',
    'https://app.example.test/b',
  ]);
  const one = await inbox.waitForLink((url) => url.endsWith('/b'));
  expect(one.link).toBe('https://app.example.test/b');
});

test('4. a code-less mail first, then the code mail: the code is returned', async ({ inbox }) => {
  await seed(inbox.name, { text: 'Welcome aboard. No code in this one.' });
  await seed(inbox.name, { text: 'Your code is 735190.' });
  const { code } = await inbox.waitForCode();
  expect(code).toBe('735190');
});

test('5. a wait parked before the mail is sent still returns it', async ({ inbox }) => {
  const pending = inbox.waitForCode({ timeoutMs: 15_000 });
  pending.catch(() => {}); // a failing gate below must not leave this unhandled
  await waitParked(inbox.name);
  await seed(inbox.name, { text: 'Your code is 906124.' });
  const { code } = await pending;
  expect(code).toBe('906124');
});

test("6. mail that reaches another test's inbox never satisfies this wait", async ({ inbox }) => {
  await seed(`${inbox.name}-other`, { text: 'Your code is 551177.' });
  const error = await inbox.waitForCode({ timeoutMs: 300 }).then(() => null, (e: unknown) => e);
  expect(error).toBeInstanceOf(SwarmailTimeout);
  expect((error as SwarmailTimeout).matched).toBe(0);
});
