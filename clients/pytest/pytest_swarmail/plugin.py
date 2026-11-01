"""The pytest plugin: the ``swarmail_inbox`` fixture.

Every test execution gets its own inbox, named so the program under test can
use the name as its SMTP AUTH username (decision 0004). The fixture never
clears an inbox and never touches another one; leftover mail is the evidence a
failed test needs.
"""

import pytest

from . import Inbox, SwarmailClient, new_inbox_name


@pytest.fixture
def swarmail_inbox() -> Inbox:
    return Inbox(SwarmailClient(), new_inbox_name())
