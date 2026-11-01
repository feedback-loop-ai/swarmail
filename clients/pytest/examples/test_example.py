"""The six example tests, the same six as clients/playwright/examples/example.spec.ts.

They run against a built ``swarmail serve`` (``SWARMAIL_URL``, default
http://127.0.0.1:8025). Test 1 sends through real SMTP with the inbox name as
the AUTH username (decision 0004; ``SWARMAIL_SMTP_ADDR``, default
127.0.0.1:1025 — example configuration, not fixture API). Every other seed is
REST ``POST .../seed``, the same parse and extraction path (decision 0010).
"""

import json
import os
import smtplib
import time
import urllib.parse
import urllib.request
from email.message import EmailMessage

import pytest

from pytest_swarmail import SwarmailAmbiguous, SwarmailTimeout

BASE = os.environ.get("SWARMAIL_URL", "http://127.0.0.1:8025").rstrip("/")


def seed(inbox, text, subject=None):
    body = {"to": f"{inbox}@example.test", "text": text}
    if subject is not None:
        body["subject"] = subject
    request = urllib.request.Request(
        f"{BASE}/api/v1/inboxes/{urllib.parse.quote(inbox, safe='')}/seed",
        data=json.dumps(body).encode(),
        headers={"content-type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=10) as res:
        assert res.status == 200


def wait_parked(inbox):
    """Gate on the server's readiness gauge, never on a sleep (decision 0020).

    The wait is parked once its inbox's row reads 1. The poll has a deadline.
    """
    row = f'swarmail_awaits_active{{inbox="{inbox}"}} 1'
    deadline = time.monotonic() + 10
    while True:
        with urllib.request.urlopen(f"{BASE}/metrics", timeout=10) as res:
            if row in res.read().decode().splitlines():
                return
        if time.monotonic() > deadline:
            raise AssertionError(f"the gauge never showed {row}")
        time.sleep(0.02)


def test_1_one_mail_with_one_code_and_one_link_both_waits_return_them(swarmail_inbox):
    host, _, port = os.environ.get("SWARMAIL_SMTP_ADDR", "127.0.0.1:1025").rpartition(":")
    message = EmailMessage()
    message["From"] = "app@example.test"
    message["To"] = f"{swarmail_inbox.name}@example.test"
    message["Subject"] = "Verify your account"
    message.set_content("Your code is 482913. Confirm here: https://app.example.test/verify?token=abc")
    with smtplib.SMTP(host, int(port), timeout=10) as smtp:
        smtp.login(swarmail_inbox.name, "any-password")  # the AUTH username is the inbox
        refused = smtp.send_message(message)  # a 250 means the mail is queryable
    assert refused == {}

    code = swarmail_inbox.wait_for_code()
    assert code.code == "482913"
    assert code.email["inbox"] == swarmail_inbox.name
    link = swarmail_inbox.wait_for_link()
    assert link.link == "https://app.example.test/verify?token=abc"
    assert link.email["id"] == code.email["id"]


def test_2_two_distinct_codes_in_one_mail_wait_for_code_is_ambiguous(swarmail_inbox):
    seed(swarmail_inbox.name, "Codes 111111 and 222222 were issued.")
    with pytest.raises(SwarmailAmbiguous) as raised:
        swarmail_inbox.wait_for_code()
    assert sorted(raised.value.candidates) == ["111111", "222222"]


def test_3_two_matching_links_across_two_mails_wait_for_link_is_ambiguous(swarmail_inbox):
    seed(swarmail_inbox.name, "First: https://app.example.test/a")
    seed(swarmail_inbox.name, "Second: https://app.example.test/b")
    with pytest.raises(SwarmailAmbiguous) as raised:
        swarmail_inbox.wait_for_link()
    assert sorted(raised.value.candidates) == [
        "https://app.example.test/a",
        "https://app.example.test/b",
    ]
    one = swarmail_inbox.wait_for_link(lambda url: url.endswith("/b"))
    assert one.link == "https://app.example.test/b"


def test_4_a_codeless_mail_first_then_the_code_mail_the_code_is_returned(swarmail_inbox):
    seed(swarmail_inbox.name, "Welcome aboard. No code in this one.")
    seed(swarmail_inbox.name, "Your code is 735190.")
    assert swarmail_inbox.wait_for_code().code == "735190"


def test_5_a_wait_parked_before_the_mail_is_sent_still_returns_it(swarmail_inbox):
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(max_workers=1) as pool:
        pending = pool.submit(swarmail_inbox.wait_for_code, timeout_ms=15000)
        wait_parked(swarmail_inbox.name)
        seed(swarmail_inbox.name, "Your code is 906124.")
        assert pending.result(timeout=20).code == "906124"


def test_6_mail_that_reaches_another_tests_inbox_never_satisfies_this_wait(swarmail_inbox):
    seed(f"{swarmail_inbox.name}-other", "Your code is 551177.")
    with pytest.raises(SwarmailTimeout) as raised:
        swarmail_inbox.wait_for_code(timeout_ms=300)
    assert raised.value.matched == 0
