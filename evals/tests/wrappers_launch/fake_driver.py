# A forge-driver/v1 stand-in for brokkr v0.15.2's `brokkr driver <family>`
# (test equipment; the header sets FAMILY, MODE_FILE and RECORD). It speaks
# the guide's envelopes, plays the seat's agent by writing the result file,
# and records what the wrapper gave it, with secret values reduced to their
# presence.
import os, sys, json, stat, pathlib, itertools
args = sys.argv[1:]
if args == ['--version']:
    print('brokkr 0.15.2')
    sys.exit(0)
# `brokkr driver <family> -- <flags>`: anything else is brokkr's usage error.
if args[:3] != ['driver', FAMILY, '--']:
    sys.stderr.write('error: unrecognized subcommand\n')
    sys.exit(2)
mode = json.loads(pathlib.Path(MODE_FILE).read_text())
env = dict(os.environ)
if sys.platform == 'darwin':
    env.pop('__CF_USER_TEXT_ENCODING', None)
for k in ('DASHSCOPE_API_KEY', 'SPARK_API_KEY'):
    if k in env:
        env[k] = '<present>'
home = pathlib.Path(env['HOME'])
fd3 = os.readlink('/proc/self/fd/3') if os.path.exists('/proc/self/fd/3') else None
rec = {'fd3': fd3, 'argv': args, 'env': env, 'cwd': os.getcwd(),
       'home': sorted([str(p.relative_to(home)), oct(stat.S_IMODE(p.lstat().st_mode))] for p in home.rglob('*'))}
def save():
    pathlib.Path(RECORD).write_text(json.dumps(rec))
ids = itertools.count()
def send(m):
    m.update({'proto': 'forge-driver/v1', 'msg_id': 'drv-%d' % next(ids)})
    sys.stdout.write(json.dumps(m) + '\n')
    sys.stdout.flush()
def recv():
    line = sys.stdin.readline()
    return json.loads(line) if line else None
if mode['mode'] == 'silent':
    save()
    sys.exit(0)
rec['hello'] = recv()
send({'type': 'capabilities', 'driver': {'claude': 'claude-code', 'codex': 'codex', 'dsh': 'deepseek-harness'}[FAMILY], 'version': '0.15.2', 'supports': ['resume'], 'probe': True})
start = recv()
rec['start'] = start
inp = start['input']
rec['charter'] = pathlib.Path(inp['role_path']).read_text()
pair = {'effect_id': start['effect_id'], 'attempt_id': start['attempt_id']}
if mode['mode'] == 'refuse':
    send(dict(pair, type='result', status='failed', result=None, error='provider refused before work'))
else:
    send(dict(pair, type='accepted', session_ref=None))
    send(dict(pair, type='checkpoint', data={'step': 'transcript', 'model': 'not reported', 'effort': 'not reported'}))
    send(dict(pair, type='checkpoint', data={'step': 'harness-started', 'harness': FAMILY, 'launch': 'cold'}))
    # A per-turn row with a larger bill than the session: it must not count.
    send(dict(pair, type='checkpoint', data={'step': mode.get('turn_step', 'seat-turn'), 'input_tokens': 999999, 'output_tokens': 999999}))
    for final in mode.get('finals', []):
        send(dict(pair, type='checkpoint', data=final))
    # The seat's agent writes its result where the contract says.
    pathlib.Path(inp['result_path']).write_text(json.dumps({'result': mode.get('result', 'done'), 'inputs': {'input_tokens': 7}, 'notes': 'n'}))
    rec['result_written'] = True
    if mode['mode'] == 'accepted_only':
        save()
        sys.exit(0)
    status = 'failed' if mode['mode'] == 'failed' else 'succeeded'
    payload = json.loads(pathlib.Path(inp['result_path']).read_text())
    payload.update({'input_tokens': 123456789, 'output_tokens': 1})
    send(dict(pair, type='result', status=status, result=payload))
rec['shutdown'] = recv()
rec['eof_after_shutdown'] = recv() is None
save()
