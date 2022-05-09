from qiniu_sdk_bindings import http_client, QiniuInvalidDomainWithPortError, QiniuInvalidIpAddrWithPortError
import unittest


class TestDomainWithPort(unittest.TestCase):
    def test_domain_with_port(self):
        d = http_client.DomainWithPort('www.qiniu.com', 8080)
        self.assertEqual(d.domain, 'www.qiniu.com')
        self.assertEqual(d.port, 8080)
        d = http_client.DomainWithPort('www.qiniu.com:8080')
        self.assertEqual(d.domain, 'www.qiniu.com')
        self.assertEqual(d.port, 8080)
        d = http_client.DomainWithPort('www.qiniu.com')
        self.assertEqual(d.domain, 'www.qiniu.com')
        self.assertEqual(d.port, None)

        with self.assertRaises(QiniuInvalidDomainWithPortError):
            http_client.DomainWithPort('127.0.0.1', 8080)

        with self.assertRaises(QiniuInvalidDomainWithPortError):
            http_client.DomainWithPort('127.0.0.1:8080')

        with self.assertRaises(QiniuInvalidDomainWithPortError):
            http_client.DomainWithPort('127.0.0.1')


class TestIpAddrWithPort(unittest.TestCase):
    def test_ip_addr_with_port(self):
        d = http_client.IpAddrWithPort('127.0.0.1', 8080)
        self.assertEqual(d.ip_addr, '127.0.0.1')
        self.assertEqual(d.port, 8080)
        d = http_client.IpAddrWithPort('127.0.0.1:8080')
        self.assertEqual(d.ip_addr, '127.0.0.1')
        self.assertEqual(d.port, 8080)
        d = http_client.IpAddrWithPort('127.0.0.1')
        self.assertEqual(d.ip_addr, '127.0.0.1')
        self.assertEqual(d.port, None)

        with self.assertRaises(QiniuInvalidIpAddrWithPortError):
            http_client.IpAddrWithPort('www.qiniu.com', 8080)

        with self.assertRaises(QiniuInvalidIpAddrWithPortError):
            http_client.IpAddrWithPort('www.qiniu.com:8080')

        with self.assertRaises(QiniuInvalidIpAddrWithPortError):
            http_client.IpAddrWithPort('www.qiniu.com')


class TestEndpoint(unittest.TestCase):
    def test_endpoint(self):
        d = http_client.Endpoint('www.qiniu.com', 8080)
        self.assertEqual(d.domain, 'www.qiniu.com')
        self.assertEqual(d.ip_addr, None)
        self.assertEqual(d.port, 8080)
        d = http_client.Endpoint('www.qiniu.com:8080')
        self.assertEqual(d.domain, 'www.qiniu.com')
        self.assertEqual(d.ip_addr, None)
        self.assertEqual(d.port, 8080)
        d = http_client.Endpoint('www.qiniu.com')
        self.assertEqual(d.domain, 'www.qiniu.com')
        self.assertEqual(d.ip_addr, None)
        self.assertEqual(d.port, None)
        d = http_client.Endpoint('127.0.0.1', 8080)
        self.assertEqual(d.domain, None)
        self.assertEqual(d.ip_addr, '127.0.0.1')
        self.assertEqual(d.port, 8080)
        d = http_client.Endpoint('127.0.0.1:8080')
        self.assertEqual(d.domain, None)
        self.assertEqual(d.ip_addr, '127.0.0.1')
        self.assertEqual(d.port, 8080)
        d = http_client.Endpoint('127.0.0.1')
        self.assertEqual(d.domain, None)
        self.assertEqual(d.ip_addr, '127.0.0.1')
        self.assertEqual(d.port, None)
