from qiniu_sdk_alpha import credential, download, http_client
from aiohttp import web
import io
import unittest
import secrets
import aiofiles


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


class TestDownloadManager(unittest.IsolatedAsyncioTestCase):
    async def test_download_manager(self):
        case = self

        rand_bytes = secrets.token_bytes(1 << 22)

        async def getfile(request):
            return web.Response(body=rand_bytes, headers={'Etag': 'fakeetag', 'X-Reqid': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/fakeobjectname', getfile)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            generator = download.UrlsSigner(
                credential.Credential('ak', 'sk'), download.StaticDomainsUrlsGenerator(
                    ['127.0.0.1:8089'], use_https=False))
            download_manager = download.DownloadManager(generator)
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                await download_manager.async_download_to_path('fakeobjectname', f.name)
                await f.seek(0, io.SEEK_SET)
                content = await f.read(-1)
                self.assertEqual(content, rand_bytes)
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                await download_manager.download_to_async_writer('fakeobjectname', f)
                await f.seek(0, io.SEEK_SET)
                content = await f.read(-1)
                self.assertEqual(content, rand_bytes)
            reader = download_manager.async_reader('fakeobjectname')
            content = await reader.readall()
            self.assertEqual(content, rand_bytes)
        finally:
            await runner.cleanup()
