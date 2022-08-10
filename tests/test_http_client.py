from qiniu_sdk_alpha import credential, http, http_client, QiniuInvalidDomainWithPortError, QiniuInvalidIpAddrWithPortError, QiniuEmptyRegionsProvider, QiniuApiCallError
from aiohttp import web
import os
import io
import aiofiles
import unittest
import fractions


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
        self.assertEqual(e.get(
            service_names=[http_client.ServiceName.Up]),
            http_client.Endpoints(
                ['192.168.1.1:8080', '192.168.1.2:8080'],
                ['192.168.2.1:8080', '192.168.2.2:8080']))
        self.assertEqual(e.get(
            service_names=[http_client.ServiceName.Rs, http_client.ServiceName.Rsf]),
            http_client.Endpoints(
                ['192.168.5.1:8080', '192.168.5.2:8080',
                    '192.168.7.1:8080', '192.168.7.2:8080'],
                ['192.168.6.1:8080', '192.168.6.2:8080', '192.168.8.1:8080', '192.168.8.2:8080']))


class TestAllRegionsProvider(unittest.IsolatedAsyncioTestCase):
    async def test_all_regions_provider(self):
        async def handler(request):
            return web.json_response(regions_response_body(), headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/regions', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            provider = http_client.AllRegionsProvider.in_memory(credential.Credential(
                'ak', 'sk'), use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            regions = await provider.async_get_all()
            self.assertEqual(len(regions), 5)
            self.assertEqual(regions[0].region_id, 'z0')
            self.assertEqual(regions[1].region_id, 'z1')
            self.assertEqual(regions[2].region_id, 'z2')
            self.assertEqual(regions[3].region_id, 'as0')
            self.assertEqual(regions[4].region_id, 'na0')
            region = await provider.async_get()
            self.assertEqual(region.region_id, 'z0')
        finally:
            await runner.cleanup()


class TestBucketRegionsQueryer(unittest.IsolatedAsyncioTestCase):
    async def test_bucket_regions_queryer(self):
        async def handler(request):
            return web.json_response(query_response_body(), headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/v4/query', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            provider = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            query = provider.query('ak', 'bucket')
            regions = await query.async_get_all()
            self.assertEqual(len(regions), 2)
            self.assertEqual(regions[0].region_id, 'z0')
            self.assertEqual(regions[1].region_id, 'z1')
            region = await query.async_get()
            self.assertEqual(region.region_id, 'z0')
        finally:
            await runner.cleanup()


class TestBucketDomainsQueryer(unittest.IsolatedAsyncioTestCase):
    async def test_bucket_domains_queryer(self):
        async def handler(request):
            return web.json_response(domains_response_body(), headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/v2/domains', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            provider = http_client.BucketDomainsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            query = provider.query(credential.Credential('ak', 'sk'), 'bucket')
            endpoints = await query.async_get()
            self.assertEqual(endpoints.preferred, [http_client.Endpoint(
                'fakedomain.1.com'), http_client.Endpoint('fakedomain.2.com')])
        finally:
            await runner.cleanup()


def regions_response_body():
    return {
        "regions": [
            {
                "id": "z0",
                "ttl": 5,
                "description": "East China",
                "io": {
                    "domains": [
                        "iovip.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload.qiniup.com",
                        "up.qiniup.com"
                    ],
                    "old": [
                        "upload.qbox.me",
                        "up.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-z0.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-z0.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-cn-east-1.qiniucs.com"
                    ],
                    "region_alias": "cn-east-1"
                }
            },
            {
                "id": "z1",
                "ttl": 5,
                "description": "North China",
                "io": {
                    "domains": [
                        "iovip-z1.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload-z1.qiniup.com",
                        "up-z1.qiniup.com"
                    ],
                    "old": [
                        "upload-z1.qbox.me",
                        "up-z1.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-z1.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-z1.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-cn-north-1.qiniucs.com"
                    ],
                    "region_alias": "cn-north-1"
                }
            },
            {
                "id": "z2",
                "ttl": 5,
                "description": "South China",
                "io": {
                    "domains": [
                        "iovip-z2.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload-z2.qiniup.com",
                        "up-z2.qiniup.com"
                    ],
                    "old": [
                        "upload-z2.qbox.me",
                        "up-z2.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-z2.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-z2.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-cn-south-1.qiniucs.com"
                    ],
                    "region_alias": "cn-south-1"
                }
            },
            {
                "id": "as0",
                "ttl": 5,
                "description": "Southeast Asia",
                "io": {
                    "domains": [
                        "iovip-as0.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload-as0.qiniup.com",
                        "up-as0.qiniup.com"
                    ],
                    "old": [
                        "upload-as0.qbox.me",
                        "up-as0.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-na0.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-na0.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-ap-southeast-1.qiniucs.com"
                    ],
                    "region_alias": "ap-southeast-1"
                }
            },
            {
                "id": "na0",
                "ttl": 5,
                "description": "North America",
                "io": {
                    "domains": [
                        "iovip-na0.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload-na0.qiniup.com",
                        "up-na0.qiniup.com"
                    ],
                    "old": [
                        "upload-na0.qbox.me",
                        "up-na0.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-na0.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-na0.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-us-north-1.qiniucs.com"
                    ],
                    "region_alias": "us-north-1"
                }
            }
        ]
    }


def query_response_body():
    return {
        "hosts": [
            {
                "region": "z0",
                "ttl": 5,
                "io": {
                    "domains": [
                          "iovip.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload.qiniup.com",
                        "up.qiniup.com"
                    ],
                    "old": [
                        "upload.qbox.me",
                        "up.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-z0.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-z0.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-cn-east-1.qiniucs.com"
                    ],
                    "region_alias": "cn-east-1"
                }
            },
            {
                "region": "z1",
                "ttl": 5,
                "io": {
                    "domains": [
                          "iovip-z1.qbox.me"
                    ]
                },
                "up": {
                    "domains": [
                        "upload-z1.qiniup.com",
                        "up-z1.qiniup.com"
                    ],
                    "old": [
                        "upload-z1.qbox.me",
                        "up-z1.qbox.me"
                    ]
                },
                "uc": {
                    "domains": [
                        "uc.qbox.me"
                    ]
                },
                "rs": {
                    "domains": [
                        "rs-z1.qbox.me"
                    ]
                },
                "rsf": {
                    "domains": [
                        "rsf-z1.qbox.me"
                    ]
                },
                "api": {
                    "domains": [
                        "api.qiniu.com"
                    ]
                },
                "s3": {
                    "domains": [
                        "s3-cn-north-1.qiniucs.com"
                    ],
                    "region_alias": "cn-north-1"
                }
            }
        ]
    }


def domains_response_body():
    return ["fakedomain.1.com", "fakedomain.2.com"]


class TestAuthorization(unittest.IsolatedAsyncioTestCase):
    async def test_authorization_sign(self):
        req = http.AsyncHttpRequest(
            url='http://127.0.0.1:8080/robots.txt',
            method='POST',
            body=b'hello world')
        cred = credential.Credential('ak', 'sk')
        auth = http_client.Authorization.v1(cred)
        await auth.async_sign(req)
        self.assertEqual(req.headers['authorization'],
                         'QBox ak:OM5YrCaVA6t1nWsDpqPOdIZ2ufA=')
        auth = http_client.Authorization.v2(cred)
        await auth.async_sign(req)
        self.assertTrue(req.headers['authorization'].startswith('Qiniu ak:'))
        auth = http_client.Authorization.download(cred)
        await auth.async_sign(req)
        self.assertTrue(req.url.startswith(
            'http://127.0.0.1:8080/robots.txt?e='))


class TestResolver(unittest.IsolatedAsyncioTestCase):
    async def test_simple_resolver(self):
        resolver = http_client.SimpleResolver()
        domains = await resolver.async_resolve('upload.qiniup.com')
        self.assertTrue(len(domains) > 0)

    async def test_trust_dns_resolver(self):
        resolver = http_client.TrustDnsResolver()
        domains = await resolver.async_resolve('upload.qiniup.com')
        self.assertTrue(len(domains) > 0)


class TestChoose(unittest.IsolatedAsyncioTestCase):
    async def test_chooser(self):
        chooser = http_client.DirectChooser()
        chosen = await chooser.async_choose(['127.0.0.1:8000', '127.0.0.1:8001', '127.0.0.1:8002'])
        self.assertEqual(
            chosen, ['127.0.0.1:8000', '127.0.0.1:8001', '127.0.0.1:8002'])

    async def test_ip_chooser(self):
        chooser = http_client.IpChooser()
        chosen = await chooser.async_choose(['127.0.0.1', '127.0.0.2', '127.0.1.1'])
        self.assertEqual(chosen, ['127.0.0.1', '127.0.0.2', '127.0.1.1'])
        try:
            provider = http_client.BucketDomainsQueryer.in_memory(
                uc_endpoints=http_client.Endpoints(['127.0.0.1']))
            query = provider.query(credential.Credential(
                'fakeak', 'fakesk'), 'fakebucket')
            await query.async_get()
            self.fail('should not be here')
        except QiniuApiCallError as e:
            await chooser.async_feedback(['127.0.0.1'], error=e)
            chosen = await chooser.async_choose(['127.0.0.1', '127.0.0.2', '127.0.1.1'])
            self.assertEqual(chosen, ['127.0.0.2', '127.0.1.1'])

    async def test_subnet_chooser(self):
        chooser = http_client.SubnetChooser()
        try:
            provider = http_client.BucketDomainsQueryer.in_memory(
                uc_endpoints=http_client.Endpoints(['127.0.0.1']))
            query = provider.query(credential.Credential(
                'fakeak', 'fakesk'), 'fakebucket')
            await query.async_get()
            self.fail('should not be here')
        except QiniuApiCallError as e:
            await chooser.async_feedback(['127.0.0.1', '127.0.0.2'], error=e)
            chosen = await chooser.async_choose(['127.0.0.1', '127.0.0.2', '127.0.1.1'])
            self.assertEqual(chosen, ['127.0.1.1'])

    async def test_never_empty_handed_chooser(self):
        chooser = http_client.NeverEmptyHandedChooser(
            http_client.IpChooser(), fractions.Fraction(1, 2))
        try:
            provider = http_client.BucketDomainsQueryer.in_memory(
                uc_endpoints=http_client.Endpoints(['127.0.0.1', '127.0.0.2', '127.0.1.1']))
            query = provider.query(credential.Credential(
                'fakeak', 'fakesk'), 'fakebucket')
            await query.async_get()
            self.fail('should not be here')
        except QiniuApiCallError as e:
            await chooser.async_feedback(['127.0.0.1', '127.0.0.2', '127.0.1.1'], error=e)
            chosen = await chooser.async_choose(['127.0.0.1', '127.0.0.2', '127.0.1.1'])
            self.assertEqual(len(chosen), 2)


class TestRetrier(unittest.IsolatedAsyncioTestCase):
    async def test_error_retrier(self):
        retrier = http_client.ErrorRetrier()
        try:
            provider = http_client.BucketDomainsQueryer.in_memory(
                uc_endpoints=http_client.Endpoints(['127.0.0.1']))
            query = provider.query(credential.Credential(
                'fakeak', 'fakesk'), 'fakebucket')
            await query.async_get()
            self.fail('should not be here')
        except QiniuApiCallError as e:
            decision = retrier.retry(
                http.HttpRequestParts(url='http://www.qiniu.com'), e)
            self.assertEqual(decision, http_client.RetryDecision.TryNextServer)

    async def test_limited_retrier(self):
        async def handler(request):
            return web.json_response({"error": "concurrency limit exceeded"}, status=573, headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/v4/query', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            provider = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            query = provider.query('ak', 'bucket')
            await query.async_get()
        except QiniuApiCallError as e:
            retried_stats = http_client.RetriedStatsInfo()
            retried_stats.increase_current_endpoint()
            retried_stats.increase_current_endpoint()

            request = http.HttpRequestParts(
                url='http://127.0.0.1:8089/v4/query')

            retrier = http_client.LimitedRetrier.limit_total(
                http_client.ErrorRetrier(), 1)
            decision = retrier.retry(request, e, retried=retried_stats)
            self.assertEqual(decision, http_client.RetryDecision.DontRetry)

            retrier = http_client.LimitedRetrier.limit_current_endpoint(
                http_client.ErrorRetrier(), 1)
            decision = retrier.retry(request, e, retried=retried_stats)
            self.assertEqual(decision, http_client.RetryDecision.TryNextServer)

        finally:
            await runner.cleanup()


class TestBackoff(unittest.IsolatedAsyncioTestCase):
    async def test_backoff(self):
        async def handler(request):
            return web.json_response({"error": "concurrency limit exceeded"}, status=573, headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/v4/query', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            provider = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            query = provider.query('ak', 'bucket')
            await query.async_get()
        except QiniuApiCallError as e:
            retried_stats = http_client.RetriedStatsInfo()
            retried_stats.increase_current_endpoint()
            retried_stats.increase_current_endpoint()

            request = http.HttpRequestParts(
                url='http://127.0.0.1:8089/v4/query')

            backoff = http_client.FixedBackoff(1000000)
            self.assertEqual(backoff.delay, 1000000)
            self.assertEqual(backoff.time_ns(
                request, e, retried=retried_stats), 1000000)

            backoff = http_client.ExponentialBackoff(2, 1000000)
            self.assertEqual(backoff.base_number, 2)
            self.assertEqual(backoff.base_delay, 1000000)
            self.assertEqual(backoff.time_ns(
                request, e, retried=retried_stats), 4000000)

            backoff = http_client.RandomizedBackoff(http_client.FixedBackoff(
                1000000), fractions.Fraction(1, 2), fractions.Fraction(3, 2))
            self.assertEqual(backoff.minification, fractions.Fraction(1, 2))
            self.assertEqual(backoff.magnification, fractions.Fraction(3, 2))
            time_ns = backoff.time_ns(request, e, retried=retried_stats)
            self.assertTrue(time_ns <= 1500000)
            self.assertTrue(time_ns >= 500000)

            backoff = http_client.LimitedBackoff(backoff, 900000, 1100000)
            self.assertEqual(backoff.min_backoff, 900000)
            self.assertEqual(backoff.max_backoff, 1100000)
            time_ns = backoff.time_ns(request, e, retried=retried_stats)
            self.assertTrue(time_ns <= 1100000)
            self.assertTrue(time_ns >= 900000)

        finally:
            await runner.cleanup()


class TestHttpClient(unittest.IsolatedAsyncioTestCase):
    async def test_get(self):
        async def handler(request):
            self.assertEqual(request.query['fakeops'], '')
            self.assertEqual(request.query['key1'], 'val1')
            self.assertEqual(request.query['key2'], 'val2')
            self.assertTrue(
                request.headers['Authorization'].startswith('Qiniu ak:'))
            return web.json_response({}, status=200, headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/getfile', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            client = http_client.HttpClient()
            resp = await client.async_call(
                'GET', http_client.Endpoints(['127.0.0.1:8089']),
                use_https=False,
                path='/getfile',
                query='fakeops',
                query_pairs=[('key1', 'val1'), ('key2', 'val2')],
                accept_json=True,
                authorization=http_client.Authorization.v2(credential.Credential('ak', 'sk')))
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(await resp.parse_json(), {})
        finally:
            await runner.cleanup()

    async def test_post_bytes(self):
        async def handler(request):
            self.assertTrue(
                request.headers['Authorization'].startswith('Qiniu ak:'))
            self.assertEqual(await request.text(), 'hello world')
            return web.json_response({}, status=200, headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.post('/post', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            client = http_client.HttpClient()
            resp = await client.async_call(
                'POST', http_client.Endpoints(['127.0.0.1:8089']),
                use_https=False,
                path='/post',
                accept_json=True,
                authorization=http_client.Authorization.v2(
                    credential.Credential('ak', 'sk')),
                bytes=b'hello world')
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(await resp.parse_json(), {})
        finally:
            await runner.cleanup()

    async def test_post_reader(self):
        async def handler(request):
            self.assertTrue(
                request.headers['Authorization'].startswith('Qiniu ak:'))
            body = await request.read()
            self.assertEqual(len(body), 1 << 20)
            return web.json_response({}, status=200, headers={'X-ReqId': 'fakereqid'})

        app = web.Application(client_max_size=1 << 30)
        app.add_routes([web.post('/postbinary', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            async with aiofiles.tempfile.TemporaryFile('wb+') as f:
                p = None
                s = None
                h = {}

                def uploading_progress(ctx, progress):
                    nonlocal p
                    p = progress

                def receive_response_status(ctx, status_code):
                    nonlocal s
                    s = status_code

                def receive_response_header(ctx, header_name, header_value):
                    nonlocal h
                    h[header_name] = header_value

                def to_choose_ips(ctx, ips):
                    self.assertEqual(ips, ['127.0.0.1:8089'])

                def ips_chosen(ctx, before, after):
                    self.assertEqual(before, ['127.0.0.1:8089'])
                    self.assertEqual(after, ['127.0.0.1:8089'])

                client = http_client.HttpClient()
                await f.write(os.urandom(1 << 20))
                await f.seek(0, io.SEEK_SET)
                resp = await client.async_call(
                    'POST', http_client.Endpoints(['127.0.0.1:8089']),
                    use_https=False,
                    path='/postbinary',
                    accept_json=True,
                    authorization=http_client.Authorization.v2(
                        credential.Credential('ak', 'sk')),
                    body=f,
                    body_len=1 << 20,
                    uploading_progress=uploading_progress,
                    receive_response_status=receive_response_status,
                    receive_response_header=receive_response_header,
                    to_choose_ips=to_choose_ips,
                    ips_chosen=ips_chosen)
                self.assertEqual(resp.status_code, 200)
                self.assertEqual(await resp.parse_json(), {})
                self.assertEqual(p.transferred_bytes, 1 << 20)
                self.assertEqual(p.total_bytes, 1 << 20)
                self.assertEqual(s, 200)
                self.assertEqual(h['content-type'],
                                 'application/json; charset=utf-8')
                self.assertTrue('Python/' in h['server'])
        finally:
            await runner.cleanup()

    async def test_post_form(self):
        async def handler(request):
            self.assertTrue(
                request.headers['Authorization'].startswith('Qiniu ak:'))
            form = await request.post()
            self.assertEqual(form['key1'], 'val1')
            self.assertEqual(form['key2'], 'val2')
            self.assertEqual(form['key3'], '')
            return web.json_response({}, status=200, headers={'X-ReqId': 'fakereqid'})

        app = web.Application(client_max_size=1 << 30)
        app.add_routes([web.post('/postform', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            client = http_client.HttpClient()
            resp = await client.async_call(
                'POST', http_client.Endpoints(['127.0.0.1:8089']),
                use_https=False,
                path='/postform',
                accept_json=True,
                authorization=http_client.Authorization.v2(
                    credential.Credential('ak', 'sk')),
                form=[('key1', 'val1'), ('key2', 'val2'), ('key3', None)])
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(await resp.parse_json(), {})
        finally:
            await runner.cleanup()

    async def test_post_multiparts(self):
        async def handler(request):
            self.assertTrue(
                request.headers['Authorization'].startswith('Qiniu ak:'))
            multipart = await request.multipart()
            for _ in range(3):
                part = await multipart.next()
                if part.name == 'file':
                    self.assertEqual(part.filename, 'test.bin')
                    self.assertEqual(
                        part.headers['content-type'], 'application/octet-stream')
                    content = await part.read()
                    self.assertEqual(len(content), 1 << 20)
                elif part.name == 'key1':
                    self.assertEqual(await part.text(), 'val1')
                elif part.name == 'key2':
                    self.assertEqual(await part.read(), b'val2')
                else:
                    self.fail('unexpected part name: ' + part.name)
            return web.json_response({}, status=200, headers={'X-ReqId': 'fakereqid'})

        app = web.Application(client_max_size=1 << 30)
        app.add_routes([web.post('/postmultipart', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            async with aiofiles.tempfile.TemporaryFile('wb+') as f:
                await f.write(os.urandom(1 << 20))
                await f.seek(0, io.SEEK_SET)
                client = http_client.HttpClient()
                resp = await client.async_call(
                    'POST', http_client.Endpoints(['127.0.0.1:8089']),
                    use_https=False,
                    path='/postmultipart',
                    accept_json=True,
                    authorization=http_client.Authorization.v2(
                        credential.Credential('ak', 'sk')),
                    multipart={'key1': 'val1', 'key2': b'val2', 'file': (f, {"file_name": "test.bin", "mime": "application/octet-stream"})})
                self.assertEqual(resp.status_code, 200)
                self.assertEqual(await resp.parse_json(), {})
        finally:
            await runner.cleanup()

    async def test_post_json(self):
        async def handler(request):
            self.assertTrue(
                request.headers['Authorization'].startswith('Qiniu ak:'))
            json = await request.json()
            self.assertEqual(
                json, {'num': -1.2, 'arr': ['str', {'dict': {'dict2': 'val'}}]})
            return web.json_response({}, status=200, headers={'X-ReqId': 'fakereqid'})

        app = web.Application(client_max_size=1 << 30)
        app.add_routes([web.post('/postjson', handler)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            client = http_client.HttpClient()
            resp = await client.async_call(
                'POST', http_client.Endpoints(['127.0.0.1:8089']),
                use_https=False,
                path='/postjson',
                accept_json=True,
                authorization=http_client.Authorization.v2(
                    credential.Credential('ak', 'sk')),
                json={'num': -1.2, 'arr': ['str', {'dict': {'dict2': 'val'}}]})
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(await resp.parse_json(), {})
        finally:
            await runner.cleanup()
