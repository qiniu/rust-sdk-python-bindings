from qiniu_sdk_python_bindings import credential
import unittest
import aiofiles
import asyncio
import io

class TestCredential(unittest.TestCase):
    def test_credential(self):
        c = get_credential()
        self.assertEqual(c.sign(b'hello'), 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
    def test_credential_sign_reader(self):
        c = get_credential()
        reader = io.BytesIO(b'hello')
        self.assertEqual(c.sign_reader(reader), 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
        reader = io.BytesIO(b'world')
        self.assertEqual(c.sign_reader(reader), 'abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ=')
    def test_credential_sign_download_url(self):
        c = get_credential()
        url = c.sign_download_url('http://www.qiniu.com/?go=1', 3600)
        self.assertTrue(url.startswith('http://www.qiniu.com/?go=1&e='))
        self.assertTrue('&token=abcdefghklmnopq' in url)
class TestCredentialProvider(unittest.TestCase):
    def test_global_credential(self):
        c = get_credential()
        credential.GlobalCredentialProvider.setup(c)
        gc = credential.GlobalCredentialProvider().get()
        self.assertEqual(gc.access_key(), ACCESS_KEY)
        self.assertEqual(gc.secret_key(), SECRET_KEY)
    def test_env_credential(self):
        c = get_credential()
        credential.EnvCredentialProvider.setup(c)
        ec = credential.EnvCredentialProvider().get()
        self.assertEqual(ec.access_key(), ACCESS_KEY)
        self.assertEqual(ec.secret_key(), SECRET_KEY)
class TestAsyncEtag(unittest.IsolatedAsyncioTestCase):
    async def test_credential_sign_reader(self):
        c = get_credential()
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'hello')
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await c.sign_async_reader(f), 'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'world')
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await c.sign_async_reader(f), 'abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ=')
class TestAsyncCredentialProvider(unittest.IsolatedAsyncioTestCase):
    async def test_global_credential(self):
        c = get_credential()
        credential.GlobalCredentialProvider.setup(c)
        global_credential = await credential.GlobalCredentialProvider().async_get()
        self.assertEqual(global_credential.access_key(), ACCESS_KEY)
        self.assertEqual(global_credential.secret_key(), SECRET_KEY)

ACCESS_KEY = 'abcdefghklmnopq'
SECRET_KEY = '1234567890'

def get_credential():
    return credential.Credential(ACCESS_KEY, SECRET_KEY)

if __name__ == '__main__':
    unittest.main()
