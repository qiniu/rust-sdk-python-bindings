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
