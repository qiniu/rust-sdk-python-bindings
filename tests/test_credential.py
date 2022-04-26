from qiniu_sdk_python_bindings import Credential
import unittest
import aiofiles
import asyncio
import io

class TestCredential(unittest.TestCase):
    def test_credential(self):
        credential = Credential('abcdefghklmnopq', '1234567890')
        self.assertEqual(credential.sign(b'hello'), 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
    def test_credential_sign_reader(self):
        credential = Credential('abcdefghklmnopq', '1234567890')
        reader = io.BytesIO(b'hello')
        self.assertEqual(credential.sign_reader(reader), 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
        reader = io.BytesIO(b'world')
        self.assertEqual(credential.sign_reader(reader), 'abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ=')
    def test_credential_sign_download_url(self):
        credential = Credential('abcdefghklmnopq', '1234567890')
        url = credential.sign_download_url('http://www.qiniu.com/?go=1', 3600)
        self.assertTrue(url.startswith('http://www.qiniu.com/?go=1&e='))
        self.assertTrue('&token=abcdefghklmnopq' in url)
class TestAsyncEtag(unittest.IsolatedAsyncioTestCase):
    async def test_credential_sign_reader(self):
        credential = Credential('abcdefghklmnopq', '1234567890')
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'hello')
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await credential.sign_async_reader(f), 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'world')
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await credential.sign_async_reader(f), 'abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ=')
