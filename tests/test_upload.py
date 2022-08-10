from qiniu_sdk_alpha import upload, credential, http_client, QiniuIoError
from aiohttp import web
import unittest
import io
import os
import secrets
import hashlib
import aiofiles
import base64
import threading


class TestConcurrencyProvider(unittest.TestCase):
    def test_concurrency_provider(self):
        self.assertEqual(upload.FixedConcurrencyProvider(5).concurrency, 5)


class TestDataPartitionProvider(unittest.TestCase):
    def test_data_partition_provider(self):
        self.assertEqual(upload.FixedDataPartitionProvider(
            5*1024*1024).part_size, 5*1024*1024)
        self.assertEqual(upload.MultiplyDataPartitionProvider(
            upload.FixedDataPartitionProvider(5*1024*1024), 4*1024*1024).part_size, 4*1024*1024)
        self.assertEqual(upload.LimitedDataPartitionProvider(
            upload.FixedDataPartitionProvider(9*1024*1024), 4*1024*1024, 8*1024*1024).part_size, 8*1024*1024)
        self.assertEqual(upload.LimitedDataPartitionProvider(
            upload.FixedDataPartitionProvider(3*1024*1024), 4*1024*1024, 8*1024*1024).part_size, 4*1024*1024)


class TestResumablePolicyProvider(unittest.TestCase):
    def test_resumable_policy_provider(self):
        self.assertEqual(upload.AlwaysSinglePart().get_policy_from_size(
            1 << 23), upload.ResumablePolicy.SinglePartUploading)
        self.assertEqual(upload.AlwaysMultiParts().get_policy_from_size(
            1 << 21), upload.ResumablePolicy.MultiPartsUploading)

        provider = upload.FixedThresholdResumablePolicy(1 << 22)
        self.assertEqual(provider.get_policy_from_size(
            1 << 21), upload.ResumablePolicy.SinglePartUploading)
        self.assertEqual(provider.get_policy_from_size(
            1 << 23), upload.ResumablePolicy.MultiPartsUploading)

        rand_bytes = secrets.token_bytes(1 << 21)
        rand_reader = io.BytesIO(rand_bytes)
        (policy, reader) = provider.get_policy_from_reader(rand_reader)
        self.assertEqual(policy, upload.ResumablePolicy.SinglePartUploading)
        self.assertEqual(reader.readall(), rand_bytes)

        rand_bytes = secrets.token_bytes(1 << 23)
        rand_reader = io.BytesIO(rand_bytes)
        (policy, reader) = provider.get_policy_from_reader(rand_reader)
        self.assertEqual(policy, upload.ResumablePolicy.MultiPartsUploading)
        self.assertEqual(reader.readall(), rand_bytes)

        provider = upload.MultiplePartitionsResumablePolicyProvider(
            upload.FixedDataPartitionProvider(4*1024*1024), 4)
        self.assertEqual(provider.get_policy_from_size(
            15*1024*1024), upload.ResumablePolicy.SinglePartUploading)
        self.assertEqual(provider.get_policy_from_size(
            17*1024*1024), upload.ResumablePolicy.MultiPartsUploading)


class TestResumableRecorder(unittest.IsolatedAsyncioTestCase):
    async def test_resumable_recorder(self):
        sha1 = hashlib.sha1()
        sha1.update(b"key")
        key = upload.SourceKey(sha1.digest())
        with self.assertRaises(QiniuIoError):
            await upload.DummyResumableRecorder().open_for_async_create_new(key)
        async with aiofiles.tempfile.TemporaryDirectory() as d:
            recorder = upload.FileSystemResumableRecorder(d)

            try:
                medium = await recorder.open_for_async_create_new(key)
                await medium.write(b"hello world\n")
                await medium.flush()

                medium = await recorder.open_for_async_append(key)
                await medium.write(b"hello world\n")
                await medium.flush()

                medium = await recorder.open_for_async_read(key)
                self.assertEqual(await medium.readall(), b"hello world\nhello world\n")
            finally:
                await recorder.async_delete(key)


class TestDataSource(unittest.IsolatedAsyncioTestCase):
    async def test_data_source(self):
        async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
            slices = []
            for _ in range(1 << 10):
                bytes = os.urandom(1 << 10)
                slices.append(bytes)
                await f.write(bytes)
            await f.seek(0, io.SEEK_SET)
            data_source = upload.AsyncFileDataSource(f.name)
            for i in range(1 << 10):
                reader = await data_source.slice(1 << 10)
                self.assertEqual(await reader.readall(), slices[i])


class TestFormUploader(unittest.IsolatedAsyncioTestCase):
    async def test_form_uploader(self):
        case = self

        async def form_upload(request):
            data = await request.post()
            case.assertTrue(data['token'].startswith('ak:'))
            case.assertEqual(data['key'], 'fakeobjectname')
            case.assertEqual(data['file'].filename, 'fakefilename')
            case.assertEqual(data['file'].content_type,
                             'application/octet-stream')
            case.assertEqual(len(data['file'].file.read()), 1 << 20)
            data['file'].file.close()

            return web.json_response({'key': 'fakekey', 'hash': 'fakehash'}, headers={'X-ReqId': 'fakereqid'})

        async def query(self):
            return web.json_response(regions_info(), headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.post('/', form_upload)])
        app.add_routes([web.get('/v4/query', query)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            queryer = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            uploader = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(
                credential.Credential('ak', 'sk'), 'fakebucket', 3600),
                use_https=False,
                queryer=queryer).form_uploader()
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                for _ in range(1 << 10):
                    await f.write(os.urandom(1 << 10))
                await f.seek(0, io.SEEK_SET)
                result = await uploader.async_upload_reader(
                    f, object_name='fakeobjectname', file_name='fakefilename')
                self.assertEqual(result['key'], 'fakekey')
                self.assertEqual(result['hash'], 'fakehash')
        finally:
            await runner.cleanup()


class TestMultiPartsUploader(unittest.IsolatedAsyncioTestCase):
    async def test_multi_parts_v1_uploader(self):
        case = self
        blocks = 0

        async def mkblk(request):
            case.assertEqual(int(request.match_info['block_size']), 1 << 22)
            data = await request.read()
            case.assertEqual(len(data), 1 << 22)
            nonlocal blocks
            blocks += 1
            return web.json_response({'ctx': '===ctx-%d===' % blocks}, headers={'X-ReqId': 'fakereqid'})

        async def mkfile(request):
            case.assertEqual(int(request.match_info['file_size']), 1 << 24)
            case.assertEqual(
                base64.urlsafe_b64decode(request.match_info['encoded_key']), b'fakeobjectname')
            case.assertEqual(
                base64.urlsafe_b64decode(request.match_info['encoded_fname']), b'fakefilename')
            data = await request.read()
            case.assertEqual(
                data, b'===ctx-1===,===ctx-2===,===ctx-3===,===ctx-4===')
            return web.json_response({'body': 'done'}, headers={'X-ReqId': 'fakereqid'})

        async def query(self):
            return web.json_response(regions_info(), headers={'X-ReqId': 'fakereqid'})

        app = web.Application(client_max_size=1 << 30)
        app.add_routes([web.post('/mkblk/{block_size}', mkblk)])
        app.add_routes(
            [web.post('/mkfile/{file_size}/key/{encoded_key}/fname/{encoded_fname}', mkfile)])
        app.add_routes([web.get('/v4/query', query)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            queryer = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            uploader = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(
                credential.Credential('ak', 'sk'), 'fakebucket', 3600),
                use_https=False, queryer=queryer).multi_parts_v1_uploader(upload.DummyResumableRecorder())
            data_partitioner = upload.FixedDataPartitionProvider(1 << 22)
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                for _ in range(1 << 12):
                    await f.write(os.urandom(1 << 12))
                await f.flush()
                inited = await uploader.async_initialize_parts(upload.AsyncFileDataSource(f.name), object_name='fakeobjectname', file_name='fakefilename')
                parts = []
                while True:
                    part = await uploader.async_upload_part(inited, data_partitioner)
                    if part != None:
                        parts.append(part)
                    else:
                        break
                response = await uploader.async_complete_part(inited, parts)
                self.assertEqual(response['body'], 'done')
        finally:
            await runner.cleanup()

    async def test_multi_parts_v2_uploader(self):
        case = self

        async def init_parts(request):
            case.assertEqual(
                request.match_info['bucket_name'], 'fakebucket')
            case.assertEqual(
                base64.urlsafe_b64decode(request.match_info['encoded_key']), b'fakeobjectname')
            return web.json_response({'uploadId': 'fakeUploadId'}, headers={'X-ReqId': 'fakereqid'})

        async def upload_part(request):
            case.assertEqual(
                request.match_info['bucket_name'], 'fakebucket')
            case.assertEqual(
                base64.urlsafe_b64decode(request.match_info['encoded_key']), b'fakeobjectname')
            data = await request.read()
            case.assertEqual(len(data), 1 << 22)
            return web.json_response({'etag': 'fakeEtag-%s' % request.match_info['part_number'], 'md5': 'fakemd5'}, headers={'X-ReqId': 'fakereqid'})

        async def complete_parts(request):
            case.assertEqual(
                request.match_info['bucket_name'], 'fakebucket')
            case.assertEqual(
                base64.urlsafe_b64decode(request.match_info['encoded_key']), b'fakeobjectname')
            data = await request.json()
            case.assertEqual(data['parts'], [
                             {'etag': 'fakeEtag-1', 'partNumber': 1}, {'etag': 'fakeEtag-2', 'partNumber': 2}, {'etag': 'fakeEtag-3', 'partNumber': 3}, {'etag': 'fakeEtag-4', 'partNumber': 4}])
            case.assertEqual(data['fname'], 'fakefilename')
            return web.json_response({'body': 'done'}, headers={'X-ReqId': 'fakereqid'})

        async def query(self):
            return web.json_response(regions_info(), headers={'X-ReqId': 'fakereqid'})

        app = web.Application(client_max_size=1 << 30)
        app.add_routes(
            [web.post('/buckets/{bucket_name}/objects/{encoded_key}/uploads', init_parts)])
        app.add_routes(
            [web.put('/buckets/{bucket_name}/objects/{encoded_key}/uploads/{upload_id}/{part_number}', upload_part)])
        app.add_routes(
            [web.post('/buckets/{bucket_name}/objects/{encoded_key}/uploads/{upload_id}', complete_parts)])
        app.add_routes([web.get('/v4/query', query)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            queryer = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            upload_manager = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(
                credential.Credential('ak', 'sk'), 'fakebucket', 3600),
                use_https=False, queryer=queryer)
            uploader = upload_manager.multi_parts_v2_uploader(
                upload.DummyResumableRecorder())
            data_partitioner = upload.FixedDataPartitionProvider(1 << 22)

            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                for _ in range(1 << 12):
                    await f.write(os.urandom(1 << 12))
                await f.flush()
                inited = await uploader.async_initialize_parts(upload.AsyncFileDataSource(f.name), object_name='fakeobjectname', file_name='fakefilename')
                parts = []
                while True:
                    part = await uploader.async_upload_part(inited, data_partitioner)
                    if part != None:
                        parts.append(part)
                    else:
                        break
                response = await uploader.async_complete_part(inited, parts)
                self.assertEqual(response['body'], 'done')

            blocks = 0
            scheduler = upload.SerialMultiPartsUploaderScheduler(uploader)
            scheduler.data_partition_provider = data_partitioner
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                for _ in range(1 << 12):
                    await f.write(os.urandom(1 << 12))
                await f.flush()
                response = await scheduler.async_upload(upload.AsyncFileDataSource(f.name), object_name='fakeobjectname', file_name='fakefilename')
                self.assertEqual(response['body'], 'done')

            blocks = 0
            scheduler = upload.ConcurrentMultiPartsUploaderScheduler(uploader)
            scheduler.data_partition_provider = data_partitioner
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                for _ in range(1 << 12):
                    await f.write(os.urandom(1 << 12))
                await f.flush()
                response = await scheduler.async_upload(upload.AsyncFileDataSource(f.name), object_name='fakeobjectname', file_name='fakefilename')
                self.assertEqual(response['body'], 'done')

            uploader = upload_manager.auto_uploader(
                resumable_recorder=upload.DummyResumableRecorder(),
                data_partition_provider=data_partitioner)
            blocks = 0
            async with aiofiles.tempfile.NamedTemporaryFile('wb+') as f:
                for _ in range(1 << 12):
                    await f.write(os.urandom(1 << 12))
                await f.flush()
                response = await uploader.async_upload_path(
                    f.name,
                    object_name='fakeobjectname',
                    file_name='fakefilename',
                    multi_parts_uploader_scheduler_prefer=upload.MultiPartsUploaderSchedulerPrefer.Concurrent,
                    multi_parts_uploader_prefer=upload.MultiPartsUploaderPrefer.V2)
                self.assertEqual(response['body'], 'done')
        finally:
            await runner.cleanup()


def regions_info():
    return {
        "hosts": [
            {
                "region": "z0",
                "ttl": 5,
                "io": {"domains": []},
                "rs": {"domains": []},
                "rsf": {"domains": []},
                "api": {"domains": []},
                "s3": {"domains": []},
                "up": {
                    "domains": [
                        "127.0.0.1:8089"
                    ]
                },
                "uc": {
                    "domains": [
                        "127.0.0.1:8089"
                    ]
                },
            },
        ]
    }
