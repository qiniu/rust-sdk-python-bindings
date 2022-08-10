from threading import Thread
from qiniu_sdk_alpha import http
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from aiohttp import web
import unittest
import os
import io
import aiofiles


class TestSyncHttpRequest(unittest.TestCase):
    def test_new_sync_http_request(self):
        req = http.SyncHttpRequest(url='http://www.qiniu.com/robots.txt', method='GET', version=http.Version.HTTP_2,
                                   headers={'x-reqid': 'fakereqid'}, body=b'hello', appended_user_agent='/python', resolved_ip_addrs=[
                                       '127.0.0.1', '127.0.0.2'])
        self.assertEqual(req.url, 'http://www.qiniu.com/robots.txt')
        self.assertEqual(req.version, http.Version.HTTP_2)
        self.assertEqual(req.method, 'GET')
        self.assertEqual(req.headers, {'x-reqid': 'fakereqid'})
        self.assertEqual(req.appended_user_agent, '/python')
        self.assertTrue(req.user_agent.endswith('/python'))
        self.assertEqual(req.resolved_ip_addrs, [
                         '127.0.0.1', '127.0.0.2'])


class TestAsyncHttpRequest(unittest.TestCase):
    def test_new_async_http_request(self):
        req = http.AsyncHttpRequest(url='http://www.qiniu.com/robots.txt', method='GET', version=http.Version.HTTP_2,
                                    headers={'x-reqid': 'fakereqid'}, body=b'hello', appended_user_agent='/python', resolved_ip_addrs=[
                                        '127.0.0.1', '127.0.0.2'])
        self.assertEqual(req.url, 'http://www.qiniu.com/robots.txt')
        self.assertEqual(req.version, http.Version.HTTP_2)
        self.assertEqual(req.method, 'GET')
        self.assertEqual(req.headers, {'x-reqid': 'fakereqid'})
        self.assertEqual(req.appended_user_agent, '/python')
        self.assertTrue(req.user_agent.endswith('/python'))
        self.assertEqual(req.resolved_ip_addrs, [
                         '127.0.0.1', '127.0.0.2'])


class TestMetrics(unittest.TestCase):
    def test_metrics(self):
        metrics = http.Metrics(total_duration_ns=1234567890)
        self.assertEqual(metrics.total_duration, 1234567890)
        metrics.total_duration = 9876543210
        self.assertEqual(metrics.total_duration, 9876543210)


class TestSyncHttpResponse(unittest.TestCase):
    def test_new_sync_http_response(self):
        response = http.SyncHttpResponse(status_code=200, headers={
                                         'content-length': '1234'},
                                         version=http.Version.HTTP_11,
                                         body=b'hello',
                                         server_ip='127.0.0.1',
                                         server_port=443)
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers, {'content-length': '1234'})
        self.assertEqual(response.version, http.Version.HTTP_11)
        self.assertEqual(response.server_ip, '127.0.0.1')
        self.assertEqual(response.server_port, 443)
        self.assertEqual(response.read(2), b'he')
        self.assertEqual(response.readall(), b'llo')


class TestAsyncHttpResponse(unittest.IsolatedAsyncioTestCase):
    async def test_new_async_http_response(self):
        response = http.AsyncHttpResponse(status_code=200, headers={
            'content-length': '1234'},
            version=http.Version.HTTP_11,
            body=b'hello',
            server_ip='127.0.0.1',
            server_port=443)
        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.headers, {'content-length': '1234'})
        self.assertEqual(response.version, http.Version.HTTP_11)
        self.assertEqual(response.server_ip, '127.0.0.1')
        self.assertEqual(response.server_port, 443)
        self.assertEqual(await response.read(2), b'he')
        self.assertEqual(await response.readall(), b'llo')


class TestSyncIsahcHttpCaller(unittest.TestCase):
    def test_sync_isahc_http_caller(self):
        class HttpHandler(BaseHTTPRequestHandler):
            def do_POST(self):
                content_length = int(self.headers['Content-Length'])
                req_body = self.rfile.read(content_length)
                self.send_response(200)
                self.send_header("Content-Type", "text/plain")
                self.send_header("Content-Length", str(content_length))
                self.send_header("X-Reqid", "fakereqid")
                self.end_headers()
                self.wfile.write(req_body)

        httpd = ThreadingHTTPServer(('127.0.0.1', 0), HttpHandler)
        httpd.allow_reuse_address = True
        server_port = httpd.server_address[1]
        thread = Thread(target=httpd.serve_forever)
        thread.start()

        try:
            req = http.SyncHttpRequest(
                url='http://127.0.0.1:%d/robots.txt' % server_port,
                method='POST',
                body=b'hello world')
            resp = http.IsahcHttpCaller().call(req)
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.headers['content-type'], 'text/plain')
            self.assertEqual(resp.headers['x-reqid'], 'fakereqid')
            self.assertEqual(resp.server_ip, '127.0.0.1')
            self.assertEqual(resp.server_port, server_port)
            self.assertEqual(resp.readall(), b'hello world')
        finally:
            httpd.shutdown()
            thread.join()


class TestAsyncIsahcHttpCaller(unittest.IsolatedAsyncioTestCase):
    async def test_async_isahc_http_caller(self):
        async def put_handler(request):
            req_body = await request.read()
            return web.Response(body=req_body, headers={'Content-Type': 'text/plain', 'X-Reqid': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.put('/robots.txt', put_handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            async with aiofiles.tempfile.TemporaryFile('wb+') as f:
                rand_bytes = os.urandom(1 << 10)
                await f.write(rand_bytes)
                await f.seek(0, io.SEEK_SET)
                req = http.AsyncHttpRequest(
                    url='http://127.0.0.1:8089/robots.txt', method='PUT', body=f, body_len=1 << 10)
                resp = await http.IsahcHttpCaller().async_call(req)
                self.assertEqual(resp.status_code, 200)
                self.assertEqual(resp.headers['content-type'], 'text/plain')
                self.assertEqual(resp.headers['x-reqid'], 'fakereqid')
                self.assertEqual(resp.server_ip, '127.0.0.1')
                self.assertEqual(resp.server_port, 8089)
                resp_body = await resp.readall()
                self.assertEqual(resp_body, rand_bytes)
        finally:
            await runner.cleanup()
