# An inert agent CLI behind the real brokkr v0.15.2 driver (test equipment;
# the header sets FAMILY and RECORD). It writes the seat's result where the
# prompt's contract says, prints the harness's own usage events, and records
# what the driver gave it with secret values reduced to their presence.
import os, sys, json, re, uuid
args = sys.argv[1:]
prompt = args[-1] if FAMILY == 'dsh' else sys.stdin.read()
lines = prompt.splitlines()
at = next(i for i, l in enumerate(lines) if 'write a JSON object to exactly this file:' in l)
result_path = next(l.strip() for l in lines[at + 1:] if l.strip())
open(result_path, 'w').write(json.dumps({'result': 'done', 'inputs': {}, 'notes': 'n'}))
env = dict(os.environ)
for k in ('DASHSCOPE_API_KEY', 'SPARK_API_KEY'):
    if k in env:
        env[k] = '<present>'
dns = {}
for p in ('/etc/resolv.conf', '/run/systemd/resolve', '/etc/nsswitch.conf', '/etc/hosts'):
    if os.path.exists(p):
        dns[p] = [os.access(p, os.R_OK), os.access(p, os.W_OK)]
rec = {'argv': args, 'env': env, 'cwd': os.getcwd(), 'dns': dns, 'result_path': result_path,
       'fd3': os.readlink('/proc/self/fd/3') if os.path.exists('/proc/self/fd/3') else None,
       'fd4': os.readlink('/proc/self/fd/4') if os.path.exists('/proc/self/fd/4') else None}
open(RECORD, 'w').write(json.dumps(rec))
def p(o):
    print(json.dumps(o), flush=True)
sid = str(uuid.uuid4())
if FAMILY == 'claude':
    usage = {'input_tokens': 10, 'output_tokens': 5, 'cache_read_input_tokens': 3, 'cache_creation_input_tokens': 2}
    p({'type': 'system', 'subtype': 'init', 'session_id': sid, 'model': 'claude-sonnet-5-5', 'cwd': '.', 'tools': [], 'mcp_servers': [], 'permissionMode': 'acceptEdits', 'apiKeySource': 'none'})
    p({'type': 'assistant', 'session_id': sid, 'message': {'id': 'msg_1', 'type': 'message', 'role': 'assistant', 'model': 'claude-sonnet-5-5', 'content': [{'type': 'text', 'text': 'ok'}], 'stop_reason': 'end_turn', 'usage': usage}})
    p({'type': 'result', 'subtype': 'success', 'is_error': False, 'num_turns': 1, 'result': 'ok', 'session_id': sid, 'total_cost_usd': 0.0125, 'duration_ms': 10, 'usage': usage, 'modelUsage': {}})
elif FAMILY == 'codex':
    p({'type': 'thread.started', 'thread_id': sid})
    p({'type': 'turn.started'})
    p({'type': 'turn.completed', 'usage': {'input_tokens': 20, 'cached_input_tokens': 8, 'output_tokens': 6}})
