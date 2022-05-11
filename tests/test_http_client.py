from qiniu_sdk_bindings import http_client, QiniuInvalidDomainWithPortError, QiniuInvalidIpAddrWithPortError, QiniuEmptyRegionsProvider
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


class TestEndpoints(unittest.TestCase):
    def test_endpoints(self):
        e = http_client.Endpoints(
            [
                http_client.Endpoint('192.168.1.1', 8080),
                ('192.168.1.2', 8080),
                '192.168.1.3:8080',
            ]
        )
        self.assertEqual(e.preferred, [
            http_client.Endpoint('192.168.1.1', 8080),
            http_client.Endpoint('192.168.1.2', 8080),
            http_client.Endpoint('192.168.1.3', 8080),
        ])
        self.assertEqual(e.alternative, [])

        e = http_client.Endpoints(
            [
                http_client.Endpoint('192.168.1.1', 8080),
                ('192.168.1.2', 8080),
                '192.168.1.3:8080',
            ],
            [
                http_client.Endpoint('192.168.2.1', 8080),
                ('192.168.2.2', 8080),
                '192.168.2.3:8080',
            ]
        )
        self.assertEqual(e.preferred, [
            http_client.Endpoint('192.168.1.1', 8080),
            http_client.Endpoint('192.168.1.2', 8080),
            http_client.Endpoint('192.168.1.3', 8080),
        ])
        self.assertEqual(e.alternative, [
            http_client.Endpoint('192.168.2.1', 8080),
            http_client.Endpoint('192.168.2.2', 8080),
            http_client.Endpoint('192.168.2.3', 8080),
        ])


class TestRegion(unittest.TestCase):
    def test_region(self):
        r = http_client.Region('z0',
                               s3_region_id='cn-east-1',
                               up_preferred_endpoints=[
                                   http_client.Endpoint('192.168.1.1', 8080),
                                   http_client.Endpoint('192.168.1.2', 8080),
                               ],
                               up_alternative_endpoints=[
                                   http_client.Endpoint('192.168.2.1', 8080),
                                   http_client.Endpoint('192.168.2.2', 8080),
                               ],
                               io_preferred_endpoints=[
                                   http_client.Endpoint('192.168.3.1', 8080),
                                   http_client.Endpoint('192.168.3.2', 8080),
                               ],
                               io_alternative_endpoints=[
                                   http_client.Endpoint('192.168.4.1', 8080),
                                   http_client.Endpoint('192.168.4.2', 8080),
                               ],
                               rs_preferred_endpoints=[
                                   '192.168.5.1:8080', '192.168.5.2:8080'],
                               rs_alternative_endpoints=[
                                   '192.168.6.1:8080', '192.168.6.2:8080'
                               ],
                               rsf_preferred_endpoints=[
                                   ('192.168.7.1', 8080), ('192.168.7.2', 8080)],
                               rsf_alternative_endpoints=[
                                   ('192.168.8.1', 8080), ('192.168.8.2', 8080)])
        self.assertEqual(r.region_id, 'z0')
        self.assertEqual(r.s3_region_id, 'cn-east-1')
        self.assertEqual(r.up_preferred_endpoints, [
            http_client.Endpoint('192.168.1.1', 8080),
            http_client.Endpoint('192.168.1.2', 8080),
        ])
        self.assertEqual(r.up_alternative_endpoints, [
            http_client.Endpoint('192.168.2.1', 8080),
            http_client.Endpoint('192.168.2.2', 8080),
        ])
        self.assertEqual(r.up, http_client.Endpoints([
            '192.168.1.1:8080',
            '192.168.1.2:8080',
        ], [
            '192.168.2.1:8080',
            '192.168.2.2:8080',
        ]))
        self.assertEqual(r.io_preferred_endpoints, [
            http_client.Endpoint('192.168.3.1', 8080),
            http_client.Endpoint('192.168.3.2', 8080),
        ])
        self.assertEqual(r.io_alternative_endpoints, [
            http_client.Endpoint('192.168.4.1', 8080),
            http_client.Endpoint('192.168.4.2', 8080),
        ])
        self.assertEqual(r.io, http_client.Endpoints([
            '192.168.3.1:8080',
            '192.168.3.2:8080',
        ], [
            '192.168.4.1:8080',
            '192.168.4.2:8080',
        ]))
        self.assertEqual(r.rs_preferred_endpoints, [
            http_client.Endpoint('192.168.5.1', 8080),
            http_client.Endpoint('192.168.5.2', 8080),
        ])
        self.assertEqual(r.rs_alternative_endpoints, [
            http_client.Endpoint('192.168.6.1', 8080),
            http_client.Endpoint('192.168.6.2', 8080),
        ])
        self.assertEqual(r.rs, http_client.Endpoints([
            '192.168.5.1:8080',
            '192.168.5.2:8080',
        ], [
            '192.168.6.1:8080',
            '192.168.6.2:8080',
        ]))
        self.assertEqual(r.rsf_preferred_endpoints, [
            http_client.Endpoint('192.168.7.1', 8080),
            http_client.Endpoint('192.168.7.2', 8080),
        ])
        self.assertEqual(r.rsf_alternative_endpoints, [
            http_client.Endpoint('192.168.8.1', 8080),
            http_client.Endpoint('192.168.8.2', 8080),
        ])
        self.assertEqual(r.rsf, http_client.Endpoints([
            '192.168.7.1:8080',
            '192.168.7.2:8080',
        ], [
            '192.168.8.1:8080',
            '192.168.8.2:8080',
        ]))


class TestRegionsProvider(unittest.TestCase):
    def test_regions_provider(self):
        r1 = http_client.Region('z0',
                                s3_region_id='cn-east-1',
                                up_preferred_endpoints=[
                                    http_client.Endpoint('192.168.1.1', 8080),
                                    http_client.Endpoint('192.168.1.2', 8080),
                                ],
                                up_alternative_endpoints=[
                                    http_client.Endpoint('192.168.2.1', 8080),
                                    http_client.Endpoint('192.168.2.2', 8080),
                                ])
        r2 = http_client.Region('z1',
                                s3_region_id='cn-east-2',
                                up_preferred_endpoints=[
                                    http_client.Endpoint('192.168.3.1', 8080),
                                    http_client.Endpoint('192.168.3.2', 8080),
                                ],
                                up_alternative_endpoints=[
                                    http_client.Endpoint('192.168.3.1', 8080),
                                    http_client.Endpoint('192.168.3.2', 8080),
                                ])
        provider = http_client.RegionsProvider([r1, r2])
        r = provider.get()
        self.assertEqual(r, r1)
        r = provider.get_all()
        self.assertEqual(r, [r1, r2])

        with self.assertRaises(QiniuEmptyRegionsProvider):
            http_client.RegionsProvider([])


class TestEndpointsProvider(unittest.TestCase):
    def test_endpoints_provider(self):
        r = http_client.Region('z0',
                               s3_region_id='cn-east-1',
                               up_preferred_endpoints=[
                                   http_client.Endpoint('192.168.1.1', 8080),
                                   http_client.Endpoint('192.168.1.2', 8080),
                               ],
                               up_alternative_endpoints=[
                                   http_client.Endpoint('192.168.2.1', 8080),
                                   http_client.Endpoint('192.168.2.2', 8080),
                               ],
                               io_preferred_endpoints=[
                                   http_client.Endpoint('192.168.3.1', 8080),
                                   http_client.Endpoint('192.168.3.2', 8080),
                               ],
                               io_alternative_endpoints=[
                                   http_client.Endpoint('192.168.4.1', 8080),
                                   http_client.Endpoint('192.168.4.2', 8080),
                               ],
                               rs_preferred_endpoints=[
                                   '192.168.5.1:8080', '192.168.5.2:8080'],
                               rs_alternative_endpoints=[
                                   '192.168.6.1:8080', '192.168.6.2:8080'
                               ],
                               rsf_preferred_endpoints=[
                                   ('192.168.7.1', 8080), ('192.168.7.2', 8080)],
                               rsf_alternative_endpoints=[
                                   ('192.168.8.1', 8080), ('192.168.8.2', 8080)])
        e = http_client.EndpointsProvider(r)
        self.assertEqual(e.get_endpoints(
            service_names=[http_client.ServiceName.Up]),
            http_client.Endpoints(
                ['192.168.1.1:8080', '192.168.1.2:8080'],
                ['192.168.2.1:8080', '192.168.2.2:8080']))
        self.assertEqual(e.get_endpoints(
            service_names=[http_client.ServiceName.Rs, http_client.ServiceName.Rsf]),
            http_client.Endpoints(
                ['192.168.5.1:8080', '192.168.5.2:8080',
                    '192.168.7.1:8080', '192.168.7.2:8080'],
                ['192.168.6.1:8080', '192.168.6.2:8080', '192.168.8.1:8080', '192.168.8.2:8080']))
