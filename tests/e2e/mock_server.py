#!/usr/bin/env python3
"""
Mock HTTP server for end-to-end testing of Trabas tunneling functionality.
This server simulates various underlying services that would be tunneled through Trabas.
"""

from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import time
import sys
import argparse


class TestHandler(BaseHTTPRequestHandler):
    """HTTP request handler for testing various scenarios"""
    
    def log_message(self, format, *args):
        """Override to reduce noise in test output"""
        pass
    
    def do_GET(self):
        """Handle GET requests"""
        if self.path == '/ping':
            self._send_text_response(200, "pong")
            
        elif self.path == '/json':
            response = {
                "message": "Hello from mock server",
                "timestamp": int(time.time()),
                "method": "GET",
                "path": self.path
            }
            self._send_json_response(200, response)
            
        elif self.path == '/slow':
            time.sleep(1)
            self._send_text_response(200, "slow response")
            
        elif self.path == '/headers':
            response = {
                "headers": dict(self.headers),
                "path": self.path
            }
            self._send_json_response(200, response)
            
        elif self.path == '/status/201':
            self._send_text_response(201, "Created")
            
        elif self.path == '/status/400':
            self._send_text_response(400, "Bad Request")
            
        elif self.path == '/status/500':
            self._send_text_response(500, "Internal Server Error")
            
        else:
            self._send_text_response(404, "Not found")
    
    def do_POST(self):
        """Handle POST requests"""
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else ""
        
        if self.path == '/echo':
            response = {
                "method": "POST",
                "path": self.path,
                "body": body,
                "headers": dict(self.headers),
                "content_length": content_length
            }
            self._send_json_response(200, response)
            
        elif self.path == '/json-echo':
            try:
                json_body = json.loads(body) if body else {}
                response = {
                    "received": json_body,
                    "headers": dict(self.headers)
                }
                self._send_json_response(200, response)
            except json.JSONDecodeError:
                self._send_json_response(400, {"error": "Invalid JSON"})
                
        else:
            self._send_text_response(404, "Not found")
    
    def do_PUT(self):
        """Handle PUT requests"""
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else ""
        
        response = {
            "method": "PUT",
            "path": self.path,
            "body": body,
            "headers": dict(self.headers)
        }
        self._send_json_response(200, response)
    
    def do_DELETE(self):
        """Handle DELETE requests"""
        response = {
            "method": "DELETE",
            "path": self.path,
            "headers": dict(self.headers)
        }
        self._send_json_response(200, response)
    
    def _send_text_response(self, status_code, message):
        """Send a plain text response"""
        self.send_response(status_code)
        self.send_header('Content-type', 'text/plain')
        self.end_headers()
        self.wfile.write(message.encode())
    
    def _send_json_response(self, status_code, data):
        """Send a JSON response"""
        self.send_response(status_code)
        self.send_header('Content-type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data, indent=2).encode())


def main():
    parser = argparse.ArgumentParser(description='Mock HTTP server for Trabas E2E tests')
    parser.add_argument('--host', default='localhost', help='Host to bind to')
    parser.add_argument('--port', type=int, default=3000, help='Port to bind to')
    args = parser.parse_args()
    
    try:
        server = HTTPServer((args.host, args.port), TestHandler)
        print(f"Mock server listening on http://{args.host}:{args.port}")
        print("Available endpoints:")
        print("  GET  /ping           - Simple ping/pong")
        print("  GET  /json           - JSON response with timestamp")
        print("  GET  /slow           - Slow response (1 second)")
        print("  GET  /headers        - Echo headers")
        print("  GET  /status/{code}  - Return specific status code")
        print("  POST /echo           - Echo request details")
        print("  POST /json-echo      - Echo JSON body")
        print("  PUT  /*              - Echo PUT request")
        print("  DELETE /*            - Echo DELETE request")
        print()
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down mock server...")
        server.shutdown()
    except Exception as e:
        print(f"Error starting server: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()
