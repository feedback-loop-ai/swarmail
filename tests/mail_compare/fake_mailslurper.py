# A stand-in for the MailSlurper executable, for the launcher's own tests
# only (never evidence): it reads the -config file the launcher wrote and
# binds the three listeners MailSlurper always starts - SMTP, the service API
# and the admin (www) listener - answering just enough for readiness.
import json
import socket
import sys
import threading

cfg = json.load(open(sys.argv[1]))


def serve(host, port, handle):
    s = socket.socket()
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind((host, port))
    s.listen(8)

    def accept():
        while True:
            c, _ = s.accept()
            threading.Thread(target=handle, args=(c,), daemon=True).start()

    threading.Thread(target=accept, daemon=True).start()


def smtp(c):
    c.sendall(b"220 Welcome to MailSlurper!\r\n")
    f = c.makefile("rb")
    for line in f:
        word = line[:4].lower()
        if word in (b"ehlo", b"helo"):
            c.sendall(b"250 Hello. How very nice to meet you!\r\n")
        else:
            break
    c.sendall(b"221 Bye\r\n")
    c.close()


def http(c):
    c.recv(4096)
    body = b'{"mailItems":[],"totalPages":0,"totalRecords":0}'
    c.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: %d\r\nConnection: close\r\n\r\n%s" % (len(body), body))
    c.close()


def www(c):
    c.close()


serve(cfg["smtpAddress"], cfg["smtpPort"], smtp)
serve(cfg["serviceAddress"], cfg["servicePort"], http)
serve(cfg["wwwAddress"], cfg["wwwPort"], www)
threading.Event().wait()
