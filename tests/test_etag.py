import qiniu_rust_bindings
import unittest
import aiofiles
import asyncio
import io
import time

class TestEtag(unittest.TestCase):
    def test_empty_etag_of(self):
        empty = io.BytesIO()
        self.assertEqual(qiniu_rust_bindings.etag_of(empty), 'Fto5o-5ea0sNMlW_75VgGJCv2AcJ')
    def test_simple_etag_of(self):
        stream = io.BytesIO()
        stream.write(b'etag')
        stream.seek(0, io.SEEK_SET)
        self.assertEqual(qiniu_rust_bindings.etag_of(stream), 'FpLiADEaVoALPkdb8tJEJyRTXoe_')
    def test_middle_size_etag_of(self):
        stream = io.BytesIO()
        stream.write(_data_of_size(1 << 20))
        stream.seek(0, io.SEEK_SET)
        self.assertEqual(qiniu_rust_bindings.etag_of(stream), 'Foyl8onxBLWeRLL5oItRJphv6i4b')
    def test_large_size_etag_of(self):
        stream = io.BytesIO()
        stream.write(_data_of_size(5 * (1 << 20)))
        stream.seek(0, io.SEEK_SET)
        self.assertEqual(qiniu_rust_bindings.etag_of(stream), 'lg-Eb5KFCuZn-cUfj_oS2PPOU9xy')
class TestAsyncEtag(unittest.IsolatedAsyncioTestCase):
    async def test_empty_etag_of(self):
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            self.assertEqual(await qiniu_rust_bindings.async_etag_of(f), 'Fto5o-5ea0sNMlW_75VgGJCv2AcJ')
    async def test_simple_etag_of(self):
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(b'etag')
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await qiniu_rust_bindings.async_etag_of(f), 'FpLiADEaVoALPkdb8tJEJyRTXoe_')
    async def test_middle_size_etag_of(self):
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(_data_of_size(1 << 20))
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await qiniu_rust_bindings.async_etag_of(f), 'Foyl8onxBLWeRLL5oItRJphv6i4b')
    async def test_large_size_etag_of(self):
        async with aiofiles.tempfile.TemporaryFile('wb+') as f:
            await f.write(_data_of_size(5 * (1 << 20)))
            await f.seek(0, io.SEEK_SET)
            self.assertEqual(await qiniu_rust_bindings.async_etag_of(f), 'lg-Eb5KFCuZn-cUfj_oS2PPOU9xy')

def _data_of_size(size):
    buf = []
    rest = size
    while rest > 0:
        add_size = min(rest, 4096)
        buf.extend(_make_fake_data()[:add_size])
        rest -= add_size
    return bytearray(''.join(buf), encoding='ascii')

def _make_fake_data():
    buf = ['b' for i in range(4096)]
    buf[0] = 'A'
    buf[4094] = '\r'
    buf[4095] = '\n'
    return buf

if __name__ == '__main__':
    unittest.main()
