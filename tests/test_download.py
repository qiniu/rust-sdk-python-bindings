from qiniu_sdk_bindings import credential, download, http_client
import unittest


class TestDownloadUrlsGenerator(unittest.TestCase):
    def test_download_urls_generator(self):
        generator = download.StaticDomainsUrlsGenerator(
            ['domain.com', 'domain2:8080', '192.168.1.1', '192.168.2.1:8080'], use_https=True)
        urls = generator.generate('fakekey')
        self.assertEqual(urls, ['https://domain.com/fakekey', 'https://domain2:8080/fakekey',
                         'https://192.168.1.1/fakekey', 'https://192.168.2.1:8080/fakekey'])

        endpoints = http_client.Endpoints(
            ['domain.com', 'domain2:8080', '192.168.1.1', '192.168.2.1:8080'])
        generator = download.EndpointsUrlGenerator(endpoints, use_https=True)
        urls = generator.generate('fakekey')
        self.assertEqual(urls, ['https://domain.com/fakekey', 'https://domain2:8080/fakekey',
                         'https://192.168.1.1/fakekey', 'https://192.168.2.1:8080/fakekey'])

        generator = download.UrlsSigner(
            credential.Credential('ak', 'sk'), generator)
        urls = generator.generate('fakekey', ttl_secs=86400)
        self.assertTrue(urls[0].startswith('https://domain.com/fakekey?'))
        self.assertTrue(urls[1].startswith('https://domain2:8080/fakekey?'))
        self.assertTrue(urls[2].startswith('https://192.168.1.1/fakekey?'))
        self.assertTrue(urls[3].startswith(
            'https://192.168.2.1:8080/fakekey?'))
