from qiniu_sdk_alpha import credential, upload_token
import unittest
import aiofiles
import asyncio
import io


class TestCredential(unittest.TestCase):
    def test_credential(self):
        c = get_credential()
        self.assertEqual(c.sign(b'hello'),
                         'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')

    def test_credential_sign_reader(self):
        c = get_credential()
        reader = io.BytesIO(b'hello')
        self.assertEqual(c.sign_reader(reader),
                         'abcdefghklmnopq:b84KVc-LroDiz0ebUANfdzSRxa0=')
        reader = io.BytesIO(b'world')
        self.assertEqual(c.sign_reader(reader),
                         'abcdefghklmnopq:VjgXt0P_nCxHuaTfiFz-UjDJ1AQ=')

    def test_credential_sign_download_url(self):
        c = get_credential()
        url = c.sign_download_url('http://www.qiniu.com/?go=1', 3600)
        self.assertTrue(url.startswith('http://www.qiniu.com/?go=1&e='))
        self.assertTrue('&token=abcdefghklmnopq' in url)

    def test_credential_authorization_v1_for_request(self):
        c = get_credential()
        authorization = c.authorization_v1_for_request(
            'http://upload.qiniup.com/', 'application/x-www-form-urlencoded', b'name=test&language=go')
        self.assertEqual(
            authorization, 'QBox abcdefghklmnopq:VlWNSauF13XCI1YGoeGMUC229lI=')

    def test_credential_authorization_v1_for_request_with_body_reader(self):
        c = get_credential()
        authorization = c.authorization_v1_for_request_with_body_reader(
            'http://upload.qiniup.com/', 'application/x-www-form-urlencoded', io.BytesIO(b'name=test&language=go'))
        self.assertEqual(
            authorization, 'QBox abcdefghklmnopq:VlWNSauF13XCI1YGoeGMUC229lI=')

    def test_credential_authorization_v2_for_request(self):
        c = get_credential()
        headers = {'Content-Type': 'application/json'}
        authorization = c.authorization_v2_for_request(
            'GET', 'http://upload.qiniup.com/', headers, b'{"name":"test"}')
        self.assertEqual(
            authorization, 'Qiniu abcdefghklmnopq:vzfDS1LpyLYKU1qLScCAsf74lCk=')

    def test_credential_authorization_v2_for_request_with_body_reader(self):
        c = get_credential()
        headers = {'Content-Type': 'application/json'}
        authorization = c.authorization_v2_for_request_with_body_reader(
            'GET', 'http://upload.qiniup.com/', headers, io.BytesIO(b'{"name":"test"}'))
        self.assertEqual(
            authorization, 'Qiniu abcdefghklmnopq:vzfDS1LpyLYKU1qLScCAsf74lCk=')


class TestCredentialProvider(unittest.TestCase):
    def test_global_credential(self):
        c = get_credential()
        credential.GlobalCredentialProvider.setup(c)
        gc = credential.GlobalCredentialProvider().get()
        self.assertEqual(gc.access_key, ACCESS_KEY)
        self.assertEqual(gc.secret_key, SECRET_KEY)
        credential.GlobalCredentialProvider.clear()

    def test_env_credential(self):
        c = get_credential()
        credential.EnvCredentialProvider.setup(c)
        ec = credential.EnvCredentialProvider().get()
        self.assertEqual(ec.access_key, ACCESS_KEY)
        self.assertEqual(ec.secret_key, SECRET_KEY)
        credential.EnvCredentialProvider.clear()

    def test_chain_credential(self):
        credential.GlobalCredentialProvider.clear()
        credential.EnvCredentialProvider.clear()
        c = credential.Credential('ak_static', 'sk_static')
        cc = credential.ChainCredentialsProvider(
            [credential.GlobalCredentialProvider(), credential.EnvCredentialProvider(), c])
        self.assertEqual(cc.get().access_key, 'ak_static')
        self.assertEqual(cc.get().secret_key, 'sk_static')
        credential.EnvCredentialProvider.setup(
            credential.Credential('ak_env', 'sk_env'))
        self.assertEqual(cc.get().access_key, 'ak_env')
        self.assertEqual(cc.get().secret_key, 'sk_env')
        credential.GlobalCredentialProvider.setup(
            credential.Credential('ak_global', 'sk_global'))
        self.assertEqual(cc.get().access_key, 'ak_global')
        self.assertEqual(cc.get().secret_key, 'sk_global')


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

    async def test_credential_authorization_v1_for_request_with_body_reader(self):
        c = get_credential()
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'name=test&language=go')
            await f.seek(0, io.SEEK_SET)
            authorization = await c.authorization_v1_for_request_with_async_body_reader('http://upload.qiniup.com/', 'application/x-www-form-urlencoded', f)
            self.assertEqual(
                authorization, 'QBox abcdefghklmnopq:VlWNSauF13XCI1YGoeGMUC229lI=')

    async def test_credential_authorization_v2_for_request_with_body_reader(self):
        c = get_credential()
        headers = {'Content-Type': 'application/json'}
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'{"name":"test"}')
            await f.seek(0, io.SEEK_SET)
            authorization = await c.authorization_v2_for_request_with_async_body_reader('GET', 'http://upload.qiniup.com/', headers, f)
            self.assertEqual(
                authorization, 'Qiniu abcdefghklmnopq:vzfDS1LpyLYKU1qLScCAsf74lCk=')


class TestAsyncCredentialProvider(unittest.IsolatedAsyncioTestCase):
    async def test_global_credential(self):
        c = get_credential()
        credential.GlobalCredentialProvider.setup(c)
        global_credential = await credential.GlobalCredentialProvider().async_get()
        self.assertEqual(global_credential.access_key, ACCESS_KEY)
        self.assertEqual(global_credential.secret_key, SECRET_KEY)


ACCESS_KEY = 'abcdefghklmnopq'
SECRET_KEY = '1234567890'


def get_credential():
    return credential.Credential(ACCESS_KEY, SECRET_KEY)


if __name__ == '__main__':
    unittest.main()
