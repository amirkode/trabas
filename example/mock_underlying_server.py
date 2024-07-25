from http.server import BaseHTTPRequestHandler, HTTPServer

HOST = 'localhost'
PORT = 8007

class PingPongHandler(BaseHTTPRequestHandler):
  def do_GET(self):
    if self.path == '/ping':
      self.send_response(200)
      self.send_header('Content-type', 'text/plain')
      self.end_headers()
      self.wfile.write(b"pong")
    else:
      self.send_error(404, 'Not found')

with HTTPServer((HOST, PORT), PingPongHandler) as server:
  print(f"Server listening on http://{HOST}:{PORT}")
  server.serve_forever()
