#!/usr/bin/env python3
"""
End-to-end tests for Trabas HTTP tunneling functionality.
Tests the complete flow: Public Request -> Server -> Client -> Underlying Service
"""

import requests
import json
import sys
import time
import argparse
from typing import Callable


class TrabasE2ETests:
    def __init__(self, server_url: str, client_id: str, timeout: int = 10):
        self.server_url = server_url.rstrip('/')
        self.client_id = client_id
        self.timeout = timeout
        self.passed = 0
        self.failed = 0
    
    def _log(self, message: str):
        """Log a message with timestamp"""
        timestamp = time.strftime("%H:%M:%S")
        print(f"[{timestamp}] {message}")
    
    def _run_test(self, test_func: Callable, test_name: str):
        """Run a single test and track results"""
        try:
            self._log(f"Running {test_name}...")
            test_func()
            self._log(f"✓ {test_name} PASSED")
            self.passed += 1
        except Exception as e:
            self._log(f"✗ {test_name} FAILED: {e}")
            self.failed += 1
    
    def test_ping_via_prefix(self):
        """Test basic tunneling via prefix path"""
        response = requests.get(f"{self.server_url}/{self.client_id}/ping", timeout=self.timeout)
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        assert response.text == "pong", f"Expected 'pong', got '{response.text}'"
    
    def test_ping_via_query_param(self):
        """Test basic tunneling via query parameter"""
        response = requests.get(f"{self.server_url}/ping?trabas_client_id={self.client_id}", timeout=self.timeout)
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        assert response.text == "pong", f"Expected 'pong', got '{response.text}'"
    
    def test_json_response(self):
        """Test JSON response handling"""
        response = requests.get(f"{self.server_url}/{self.client_id}/json", timeout=self.timeout)
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        
        data = response.json()
        assert "message" in data, "Response should contain 'message' field"
        assert data["message"] == "Hello from mock server", f"Unexpected message: {data['message']}"
        assert "timestamp" in data, "Response should contain 'timestamp' field"
    
    def test_post_request(self):
        """Test POST request tunneling"""
        payload = {"test": "data", "number": 42, "nested": {"key": "value"}}
        response = requests.post(
            f"{self.server_url}/{self.client_id}/echo",
            json=payload,
            timeout=self.timeout
        )
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        
        data = response.json()
        assert data["method"] == "POST", f"Expected POST, got {data['method']}"
        assert data["path"] == "/echo", f"Expected /echo, got {data['path']}"
        
        # Verify the body was transmitted correctly
        received_body = json.loads(data["body"])
        assert received_body == payload, "POST body not transmitted correctly"
    
    def test_put_request(self):
        """Test PUT request tunneling"""
        payload = {"action": "update", "id": 123}
        response = requests.put(
            f"{self.server_url}/{self.client_id}/resource/123",
            json=payload,
            timeout=self.timeout
        )
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        
        data = response.json()
        assert data["method"] == "PUT", f"Expected PUT, got {data['method']}"
    
    def test_delete_request(self):
        """Test DELETE request tunneling"""
        response = requests.delete(f"{self.server_url}/{self.client_id}/resource/123", timeout=self.timeout)
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        
        data = response.json()
        assert data["method"] == "DELETE", f"Expected DELETE, got {data['method']}"
    
    def test_headers_forwarding(self):
        """Test that headers are properly forwarded"""
        headers = {
            "X-Custom-Header": "test-value",
            "User-Agent": "TrabasE2ETest/1.0"
        }
        response = requests.get(
            f"{self.server_url}/{self.client_id}/headers",
            headers=headers,
            timeout=self.timeout
        )
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        
        data = response.json()
        received_headers = data["headers"]
        
        # Check that our custom headers were forwarded
        assert "X-Custom-Header" in received_headers, "Custom header not forwarded"
        assert received_headers["X-Custom-Header"] == "test-value", "Custom header value incorrect"
    
    def test_different_status_codes(self):
        """Test that different HTTP status codes are properly forwarded"""
        test_cases = [
            (201, "Created"),
            (400, "Bad Request"),
            (500, "Internal Server Error")
        ]
        
        for expected_status, expected_text in test_cases:
            response = requests.get(
                f"{self.server_url}/{self.client_id}/status/{expected_status}",
                timeout=self.timeout
            )
            assert response.status_code == expected_status, \
                f"Expected {expected_status}, got {response.status_code}"
            assert response.text == expected_text, \
                f"Expected '{expected_text}', got '{response.text}'"
    
    def test_slow_request(self):
        """Test handling of slow requests"""
        start_time = time.time()
        response = requests.get(f"{self.server_url}/{self.client_id}/slow", timeout=15)
        duration = time.time() - start_time
        
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        assert response.text == "slow response", f"Expected 'slow response', got '{response.text}'"
        assert duration >= 1.0, f"Request should take at least 1 second, took {duration:.2f}s"
        assert duration < 5.0, f"Request took too long: {duration:.2f}s"
    
    def test_nonexistent_client(self):
        """Test request to non-existent client ID"""
        try:
            response = requests.get(f"{self.server_url}/nonexistent-client/ping", timeout=5)
            # If we get a response, it should be an error status
            assert response.status_code >= 400, \
                f"Expected error status for nonexistent client, got {response.status_code}"
        except requests.exceptions.Timeout:
            # Timeout is also acceptable for nonexistent clients
            pass
        except requests.exceptions.ConnectionError:
            # Connection error is also acceptable for nonexistent clients
            pass
    
    def test_large_payload(self):
        """Test handling of larger payloads"""
        large_data = {
            "data": "x" * 10000,  # 10KB of data
            "numbers": list(range(1000)),
            "metadata": {
                "size": "large",
                "test": True
            }
        }
        
        response = requests.post(
            f"{self.server_url}/{self.client_id}/json-echo",
            json=large_data,
            timeout=self.timeout
        )
        assert response.status_code == 200, f"Expected 200, got {response.status_code}"
        
        data = response.json()
        assert "received" in data, "Response should contain 'received' field"
        assert data["received"]["data"] == large_data["data"], "Large payload not transmitted correctly"
        assert len(data["received"]["numbers"]) == 1000, "Array data not transmitted correctly"
    
    def run_all_tests(self):
        """Run all E2E tests"""
        self._log("Starting Trabas End-to-End Tests")
        self._log(f"Server URL: {self.server_url}")
        self._log(f"Client ID: {self.client_id}")
        self._log(f"Timeout: {self.timeout}s")
        self._log("-" * 50)
        
        # Wait for services to stabilize
        time.sleep(2)
        
        # Define all tests
        tests = [
            (self.test_ping_via_prefix, "Ping via prefix path"),
            (self.test_ping_via_query_param, "Ping via query parameter"),
            (self.test_json_response, "JSON response handling"),
            (self.test_post_request, "POST request tunneling"),
            (self.test_put_request, "PUT request tunneling"),
            (self.test_delete_request, "DELETE request tunneling"),
            (self.test_headers_forwarding, "Headers forwarding"),
            (self.test_different_status_codes, "Different status codes"),
            (self.test_slow_request, "Slow request handling"),
            (self.test_large_payload, "Large payload handling"),
            (self.test_nonexistent_client, "Non-existent client ID"),
        ]
        
        # Run all tests
        for test_func, test_name in tests:
            self._run_test(test_func, test_name)
        
        # Print summary
        self._log("-" * 50)
        self._log(f"Test Results: {self.passed} passed, {self.failed} failed")
        
        if self.failed > 0:
            self._log("Some tests failed!")
            return False
        else:
            self._log("All tests passed!")
            return True


def main():
    parser = argparse.ArgumentParser(description='Run Trabas E2E tests')
    parser.add_argument('--server-url', default='http://localhost:8001', 
                       help='Trabas server URL')
    parser.add_argument('--client-id', default='e2e-test-client', 
                       help='Client ID to use for testing')
    parser.add_argument('--timeout', type=int, default=10, 
                       help='Request timeout in seconds')
    args = parser.parse_args()
    
    tester = TrabasE2ETests(args.server_url, args.client_id, args.timeout)
    success = tester.run_all_tests()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
