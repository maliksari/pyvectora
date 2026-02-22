from pyvectora import Response, Request

def test_response_headers():
    resp = Response.json({"ok": True}).with_header("X-Test", "1")
    assert resp.headers["X-Test"] == "1"

def test_request_text():
    req = Request(body="hello")
    assert req.text == "hello"
