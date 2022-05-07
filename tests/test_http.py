from qiniu_sdk_bindings import http
import unittest


class TestSyncHttpRequest(unittest.TestCase):
    def test_build_sync_http_request(self):
        builder = http.SyncHttpRequestBuilder()
        builder.url('http://www.qiniu.com/robots.txt')
        builder.method('GET')
        builder.version(http.Version.HTTP_2)
        builder.headers({'x-reqid': 'fakereqid'})
        builder.body(b'hello')
        builder.appended_user_agent('/python')
        builder.resolved_ip_addrs(['127.0.0.1', '127.0.0.2'])
        req = builder.build()
        self.assertEqual(req.url, 'http://www.qiniu.com/robots.txt')
        self.assertEqual(req.version, http.Version.HTTP_2)
        self.assertEqual(req.method, 'GET')
        self.assertEqual(req.headers, {'x-reqid': 'fakereqid'})
        self.assertEqual(req.appended_user_agent, '/python')
        self.assertTrue(req.user_agent.endswith('/python'))
        self.assertEqual(req.resolved_ip_addrs, [
                         '127.0.0.1', '127.0.0.2'])
        req.url = 'http://developer.qiniu.com/robots.txt'
        req.version = http.Version.HTTP_3
        req.appended_user_agent = '/python/3.8.0'
        self.assertEqual(req.url, 'http://developer.qiniu.com/robots.txt')
        self.assertEqual(req.version, http.Version.HTTP_3)
        self.assertEqual(req.appended_user_agent, '/python/3.8.0')
        self.assertTrue(req.user_agent.endswith('/python/3.8.0'))

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
    def test_build_async_http_request(self):
        builder = http.AsyncHttpRequestBuilder()
        builder.url('http://www.qiniu.com/robots.txt')
        builder.method('GET')
        builder.version(http.Version.HTTP_2)
        builder.headers({'x-reqid': 'fakereqid'})
        builder.body(b'hello')
        builder.appended_user_agent('/python')
        builder.resolved_ip_addrs(['127.0.0.1', '127.0.0.2'])
        req = builder.build()
        self.assertEqual(req.url, 'http://www.qiniu.com/robots.txt')
        self.assertEqual(req.version, http.Version.HTTP_2)
        self.assertEqual(req.method, 'GET')
        self.assertEqual(req.headers, {'x-reqid': 'fakereqid'})
        self.assertEqual(req.appended_user_agent, '/python')
        self.assertTrue(req.user_agent.endswith('/python'))
        self.assertEqual(req.resolved_ip_addrs, [
                         '127.0.0.1', '127.0.0.2'])
        req.url = 'http://developer.qiniu.com/robots.txt'
        req.version = http.Version.HTTP_3
        req.appended_user_agent = '/python/3.8.0'
        self.assertEqual(req.url, 'http://developer.qiniu.com/robots.txt')
        self.assertEqual(req.version, http.Version.HTTP_3)
        self.assertEqual(req.appended_user_agent, '/python/3.8.0')
        self.assertTrue(req.user_agent.endswith('/python/3.8.0'))

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
        metrics = http.Metrics(total_duration=1234567890)
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
        req = http.SyncHttpRequest(
            url='https://www.qiniu.com/robots.txt', method='GET')
        resp = http.IsahcHttpCaller().call(req)
        self.assertEqual(resp.status_code, 200)
        self.assertEqual(resp.headers['content-type'], 'text/plain')
        self.assertEqual(resp.server_port, 443)
        self.assertTrue(b'Disallow: /' in resp.readall())


class TestAsyncIsahcHttpCaller(unittest.IsolatedAsyncioTestCase):
    async def test_async_isahc_http_caller(self):
        req = http.AsyncHttpRequest(
            url='https://www.qiniu.com/robots.txt', method='GET')
        resp = await http.IsahcHttpCaller().async_call(req)
        self.assertEqual(resp.status_code, 200)
        self.assertEqual(resp.headers['content-type'], 'text/plain')
        self.assertEqual(resp.server_port, 443)
        self.assertTrue(b'Disallow: /' in await resp.readall())
