# A stand-in for `brokkr driver <family> -- <argv...>` (decision 0116). It
# records what it was launched with, then plays one of three parts:
#   passthrough  exec the family CLI it was handed, argv unchanged
#   oracle       act as the CLI would: read the helper server's environment
#                from the MCP config the wrapper wrote and run the built-in
#                oracle with it (the eight EVAL inputs reach a real CLI the
#                same way, through its helper server)
#   idle         do nothing and exit 0
import os, sys, json, pathlib
argv = sys.argv[1:]
assert argv[1] == '--', argv
record = {'argv': argv, 'env_keys': sorted(os.environ), 'cwd': os.getcwd()}
pathlib.Path(RECORD).write_text(json.dumps(record))
if MODE == 'passthrough':
    os.execv(argv[2], argv[2:])
elif MODE == 'oracle':
    home = pathlib.Path(os.environ['HOME'])
    env = json.loads((home / 'mcp.json').read_text())['mcpServers']['fixture']['env']
    os.execve(EVALS_BIN, [EVALS_BIN, 'agent', 'oracle'], env)
sys.exit(0)
