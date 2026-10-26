import os, signal, socket, json, sys
# This executable has already replaced the wrapper in its registered group.
print(CANARY,flush=True)
print(CANARY,file=sys.stderr,flush=True)
r,w=os.pipe()
child=os.fork()
if child == 0:
    os.close(r)
    signal.signal(signal.SIGTERM,signal.SIG_IGN)
    signal.signal(signal.SIGINT,signal.SIG_IGN)
    grandchild=os.fork()
    if grandchild == 0:
        os.write(w,(str(os.getpid())+'\n').encode())
        os.close(w)
        while True: signal.pause()
    os.close(w)
    while True: signal.pause()
os.close(w)
with os.fdopen(r) as f: grandchild=int(f.readline())
s=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
s.connect(SOCKET)
s.sendall((json.dumps({'pids':[os.getpid(),child,grandchild],'home':os.environ['HOME']})+'\n').encode())
s.recv(100)
sys.exit(7 if ENDING == 'nonzero' else 0)
