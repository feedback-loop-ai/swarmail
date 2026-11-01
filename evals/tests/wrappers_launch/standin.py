import os, sys, json, pathlib, stat, tomllib, datetime
home = pathlib.Path(os.environ['HOME'])
# Check every private inode before reading its payload.
for p in [home, *home.rglob('*')]:
    assert stat.S_IMODE(p.stat().st_mode) == (0o700 if p.is_dir() else 0o600)
args = sys.argv[1:]
env = dict(os.environ)
# On macOS this interpreter's own CoreFoundation sets __CF_USER_TEXT_ENCODING
# at startup; it never comes from the wrapper (the Linux run proves the map).
if sys.platform == 'darwin':
    env.pop('__CF_USER_TEXT_ENCODING', None)
if FAMILY == 'claude':
    auth = json.loads((home/'.claude/.credentials.json').read_text())
    assert auth == {'claudeAiOauth': {'accessToken': CANARY, 'expiresAt': 1999999999999, 'scopes': ['user:inference']}}
    config = json.loads((home/'mcp.json').read_text())
elif FAMILY == 'codex':
    auth = json.loads((home/'.codex/auth.json').read_text())
    assert set(auth) == {'auth_mode','OPENAI_API_KEY','tokens','last_refresh'}
    assert auth['auth_mode'] == 'chatgpt' and auth['OPENAI_API_KEY'] is None
    assert auth['tokens'] == {'id_token':CANARY,'access_token':CANARY,'refresh_token':'unavailable','account_id':'synthetic-account'}
    datetime.datetime.fromisoformat(auth['last_refresh'])
    # Parse the complete TOML document independently of Rust's encoder.
    overrides = [args[i+1] for i,a in enumerate(args) if a == '-c']
    config = tomllib.loads('\n'.join(overrides))
else:
    config = [json.loads(line[2:]) for line in (home/'seat.yml').read_text().splitlines()]
    provider = config[1]['config']['provider']
    key, token = ('DASHSCOPE_API_KEY',CANARY) if provider == 'dashscope' else ('SPARK_API_KEY','SPARK-CANARY')
    assert env.pop(key) == token
    assert 'DASHSCOPE_API_KEY' not in env and 'SPARK_API_KEY' not in env
assert CANARY not in json.dumps(args)
assert 'SOURCE-REFRESH-NEVER-FORWARDED' not in json.dumps(config)
print(CANARY, flush=True)
print(CANARY, file=sys.stderr, flush=True)
result = {'argv':args, 'stdin':sys.stdin.read(), 'env':env, 'cwd':os.getcwd(), 'config':config}
assert CANARY not in json.dumps(result)
pathlib.Path(CAPTURE).write_text(json.dumps(result))
