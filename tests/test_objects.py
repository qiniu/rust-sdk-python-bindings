from qiniu_sdk_alpha import objects, credential, http_client
from aiohttp import web
import unittest
import base64
import time


class TestObjectsOperation(unittest.IsolatedAsyncioTestCase):
    async def test_object_operation(self):
        case = self

        async def query(self):
            return web.json_response(regions_info(), headers={'X-ReqId': 'fakereqid'})

        async def stat(self):
            case.assertEqual(
                bytes(self.match_info['entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            return web.json_response({"fsize": 1024, "hash": 'fakehash'}, headers={'X-ReqId': 'fakereqid'})

        async def copy(self):
            case.assertEqual(
                bytes(self.match_info['from_entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            case.assertEqual(
                bytes(self.match_info['to_entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket2:fakekey2'))
            case.assertEqual(
                self.match_info['force'],
                'false')
            return web.json_response({}, headers={'X-ReqId': 'fakereqid'})

        async def move(self):
            case.assertEqual(
                bytes(self.match_info['from_entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            case.assertEqual(
                bytes(self.match_info['to_entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket2:fakekey2'))
            case.assertEqual(
                self.match_info['force'],
                'true')
            return web.json_response({}, headers={'X-ReqId': 'fakereqid'})

        async def delete(self):
            case.assertEqual(
                bytes(self.match_info['entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            return web.json_response({}, headers={'X-ReqId': 'fakereqid'})

        async def restoreAr(self):
            case.assertEqual(
                bytes(self.match_info['entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            case.assertEqual(self.match_info['afterDays'], '7')
            return web.json_response({}, headers={'X-ReqId': 'fakereqid'})

        async def chtype(self):
            case.assertEqual(
                bytes(self.match_info['entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            case.assertEqual(self.match_info['type'], '1')
            return web.json_response({}, headers={'X-ReqId': 'fakereqid'})

        async def chstatus(self):
            case.assertEqual(
                bytes(self.match_info['entry'], 'utf-8'),
                base64.urlsafe_b64encode(b'fakebucket:fakekey'))
            case.assertEqual(self.match_info['status'], '1')
            return web.json_response({}, headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/stat/{entry}', stat)])
        app.add_routes(
            [web.post('/copy/{from_entry}/{to_entry}/force/{force}', copy)])
        app.add_routes(
            [web.post('/move/{from_entry}/{to_entry}/force/{force}', move)])
        app.add_routes([web.post('/delete/{entry}', delete)])
        app.add_routes(
            [web.post('/restoreAr/{entry}/freezeAfterDays/{afterDays}', restoreAr)])
        app.add_routes(
            [web.post('/chtype/{entry}/type/{type}', chtype)])
        app.add_routes(
            [web.post('/chstatus/{entry}/status/{status}', chstatus)])
        app.add_routes([web.get('/v4/query', query)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            queryer = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            objects_manager = objects.ObjectsManager(credential.Credential(
                'ak', 'sk'), use_https=False, queryer=queryer)
            bucket = objects_manager.bucket('fakebucket')
            self.assertEqual(bucket.name, 'fakebucket')
            resp = await bucket.stat_object('fakekey').async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body['fsize'], 1024)
            self.assertEqual(resp.body['hash'], 'fakehash')
            resp = await bucket.copy_object_to('fakekey', 'fakebucket2', 'fakekey2').async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body, {})
            resp = await bucket.move_object_to('fakekey', 'fakebucket2', 'fakekey2', force=True).async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body, {})
            resp = await bucket.delete_object('fakekey').async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body, {})
            resp = await bucket.restore_archived_object('fakekey', 7).async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body, {})
            resp = await bucket.set_object_type('fakekey', 1).async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body, {})
            resp = await bucket.modify_object_status('fakekey', True).async_call()
            self.assertEqual(resp.status_code, 200)
            self.assertEqual(resp.body, {})
        finally:
            await runner.cleanup()

    async def test_objects_list(self):
        case = self

        async def query(self):
            return web.json_response(regions_info(), headers={'X-ReqId': 'fakereqid'})

        async def list(self):
            case.assertEqual(self.query.get('bucket'), 'fakebucket')
            case.assertEqual(self.query.get('limit'), '1000')
            if self.query.get('marker') == 'fakemarker':
                return web.json_response({
                    "marker": "",
                    "items": [{
                        "key": "fakeobj3",
                        "put_time": generate_put_time(),
                        "hash": "fakeobj3hash",
                        "fsize": 3,
                        "mime_type": "text/plain",
                    }, {
                        "key": "fakeobj4",
                        "put_time": generate_put_time(),
                        "hash": "fakeobj4hash",
                        "fsize": 4,
                        "mime_type": "text/plain",
                    }]
                }, headers={'X-ReqId': 'fakereqid'})
            else:
                return web.json_response({
                    "marker": "fakemarker",
                    "items": [{
                        "key": "fakeobj1",
                        "put_time": generate_put_time(),
                        "hash": "fakeobj1hash",
                        "fsize": 1,
                        "mime_type": "text/plain",
                    }, {
                        "key": "fakeobj2",
                        "put_time": generate_put_time(),
                        "hash": "fakeobj2hash",
                        "fsize": 2,
                        "mime_type": "text/plain",
                    }]
                }, headers={'X-ReqId': 'fakereqid'})

        def generate_put_time():
            return int(time.time_ns()/100)

        app = web.Application()
        app.add_routes([web.get('/v4/query', query)])
        app.add_routes([web.get('/list', list)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            queryer = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            objects_manager = objects.ObjectsManager(credential.Credential(
                'ak', 'sk'), use_https=False, queryer=queryer)
            bucket = objects_manager.bucket('fakebucket')
            idx = 0
            async for object in bucket.list(version=objects.ListVersion.V1):
                idx += 1
                self.assertEqual(object['fsize'], idx)
                self.assertEqual(object['key'], 'fakeobj%d' % idx)
        finally:
            await runner.cleanup()

    async def test_objects_operation(self):
        case = self

        async def query(self):
            return web.json_response(regions_info(), headers={'X-ReqId': 'fakereqid'})

        async def batch(self):
            data = await self.post()
            responses = []
            idx = 0
            for op in data.getall('op'):
                idx += 1
                case.assertTrue(op.startswith('stat/'))
                base64ed_op = op[len('stat/'):]
                op = base64.urlsafe_b64decode(base64ed_op).decode('utf-8')
                case.assertTrue(op.startswith('fakebucket:'))
                key = op[len('fakebucket:'):]
                case.assertEqual('object_%d' % idx, key)
                responses.append(
                    {'code': 200, 'data': {'fsize': 1024, 'hash': 'fakehash'}})
            return web.json_response(responses, headers={'X-ReqId': 'fakereqid'})

        app = web.Application()
        app.add_routes([web.get('/v4/query', query)])
        app.add_routes([web.post('/batch', batch)])
        runner = web.AppRunner(app)
        await runner.setup()
        site = web.TCPSite(runner, '127.0.0.1', 8089)
        await site.start()

        try:
            queryer = http_client.BucketRegionsQueryer.in_memory(
                use_https=False, uc_endpoints=http_client.Endpoints(['127.0.0.1:8089']))
            objects_manager = objects.ObjectsManager(credential.Credential(
                'ak', 'sk'), use_https=False, queryer=queryer)
            bucket = objects_manager.bucket('fakebucket')
            ops = bucket.batch_ops([
                bucket.stat_object('object_1'),
                bucket.stat_object('object_2'),
                bucket.stat_object('object_3'),
            ])
            count = 0
            async for result in ops:
                self.assertEqual(
                    result.data, {'fsize': 1024, 'hash': 'fakehash'})
                count += 1
            self.assertEqual(count, 3)
        finally:
            await runner.cleanup()


def regions_info():
    return {
        "hosts": [
            {
                "region": "z0",
                "ttl": 5,
                "up": {"domains": []},
                "io": {"domains": []},
                "api": {"domains": []},
                "s3": {"domains": []},
                "uc": {
                    "domains": [
                        "127.0.0.1:8089"
                    ]
                },
                "rs": {
                    "domains": [
                        "127.0.0.1:8089"
                    ]
                },
                "rsf": {
                    "domains": [
                        "127.0.0.1:8089"
                    ]
                },
            },
        ]
    }
