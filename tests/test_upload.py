from qiniu_sdk_bindings import upload, QiniuIoError
import unittest
import io
import os
import secrets
import hashlib
import aiofiles


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
