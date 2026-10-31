import type { TestType } from '@playwright/test';

export { expect } from '@playwright/test';

export interface EmailAddress {
  address: string;
  name?: string;
}

/**
 * The REST `Email` object, untouched (see /openapi.json, components.Email).
 * The server leaves an empty `cc`, `recipients`, `links` or `codes` out of the
 * JSON, so those are optional here.
 */
export interface Email {
  id: string;
  inbox: string;
  from?: EmailAddress;
  envelope_from: string | null;
  to: EmailAddress[];
  cc?: EmailAddress[];
  recipients?: string[];
  subject?: string | null;
  received_at: string;
  received_ms: number;
  size: number;
  text: string | null;
  html: string | null;
  links?: string[];
  codes?: string[];
  message_id?: string;
  in_reply_to?: string;
  references?: string[];
}

export interface WaitOptions {
  /** Milliseconds to wait; default 5000, the server's own default. */
  timeoutMs?: number;
  to?: string;
  from?: string;
  subject?: string;
  /** Forwarded verbatim; a stored mail's `received_ms` is inclusive. */
  sinceMs?: number;
}

export interface CodeMatch {
  code: string;
  email: Email;
}

export interface LinkMatch {
  link: string;
  email: Email;
}

export interface Inbox {
  /** `t` + 32 hex digits; use it as the SMTP AUTH username. */
  readonly name: string;
  waitForCode(options?: WaitOptions): Promise<CodeMatch>;
  waitForLink(match?: ((url: string) => boolean) | null, options?: WaitOptions): Promise<LinkMatch>;
}

export class SwarmailError extends Error {}
export class SwarmailTimeout extends SwarmailError {
  inbox: string;
  filters: Omit<WaitOptions, 'timeoutMs'>;
  timeoutMs: number;
  matched: number;
}
export class SwarmailAmbiguous extends SwarmailError {
  inbox: string;
  filters: Omit<WaitOptions, 'timeoutMs'>;
  candidates: string[];
}
export class SwarmailHttpError extends SwarmailError {
  /** The HTTP status, or 0 for a transport failure. */
  status: number;
  body: string;
}

export class SwarmailClient {
  constructor(options?: { baseUrl?: string });
  readonly baseUrl: string;
  waitForCode(inbox: string, options?: WaitOptions): Promise<CodeMatch>;
  waitForLink(inbox: string, match?: ((url: string) => boolean) | null, options?: WaitOptions): Promise<LinkMatch>;
}

export function newInboxName(): string;

export const test: TestType<{ inbox: Inbox }, {}>;
