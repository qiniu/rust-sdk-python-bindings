from qiniu_sdk_bindings import credential, http, http_client, QiniuInvalidDomainWithPortError, QiniuInvalidIpAddrWithPortError, QiniuEmptyRegionsProvider
from aiohttp import web
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
        self.assertEqual(req.headers['authorization'],
                         'Qiniu ak:_QfaED-dau-Eh86sxUV_SvlE6ws=')
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
