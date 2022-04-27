from qiniu_sdk_bindings import upload_token
import unittest


class TestUploadPolicy(unittest.TestCase):
    def test_bucket_policy(self):
        policy = upload_token.UploadPolicy.new_for_bucket('test-bucket', 3600)
        self.assertEqual(policy.bucket(), 'test-bucket')

    def test_object_policy(self):
        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        self.assertEqual(policy.bucket(), 'test-bucket')
        self.assertEqual(policy.key(), 'test-object')
        self.assertFalse(policy.use_prefixal_object_key())

    def test_object_prefix_policy(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600)
        self.assertEqual(policy.bucket(), 'test-bucket')
        self.assertEqual(policy.key(), 'test-object')
        self.assertTrue(policy.use_prefixal_object_key())

    def test_json_convertion(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600)
        new_policy = upload_token.UploadPolicy.from_json(policy.as_json())
        self.assertEqual(new_policy.bucket(), 'test-bucket')
        self.assertEqual(new_policy.key(), 'test-object')
        self.assertTrue(new_policy.use_prefixal_object_key())

    def test_insert_only(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600, insertOnly=1)
        self.assertTrue(policy.is_insert_only())
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600, insertOnly=0)
        self.assertFalse(policy.is_insert_only())

    def test_mime_detection(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600, detectMime=1)
        self.assertTrue(policy.mime_detection_enabled())
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600, detectMime=0)
        self.assertFalse(policy.mime_detection_enabled())

    def test_return_url(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600, returnUrl='http://www.qiniu.com')
        self.assertEqual(policy.return_url(), 'http://www.qiniu.com')
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600,
            returnUrl='http://www.qiniu.com',
            returnBody='{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')
        self.assertEqual(policy.return_url(), 'http://www.qiniu.com')
        self.assertEqual(policy.return_body(
        ), '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')

    def test_callback_urls(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600,
            callbackUrl='http://cb1.com;http://cb2.com',
            callbackBody='{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')
        self.assertEqual(policy.callback_urls(), [
            'http://cb1.com', 'http://cb2.com'])
        self.assertEqual(policy.callback_body(
        ), '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')

    def test_save_key(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600,
            saveKey='test-save-key', forceSaveKey=False)
        self.assertEqual(policy.save_key(), 'test-save-key')
        self.assertFalse(policy.is_save_key_forced())
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600,
            saveKey='test-save-key', forceSaveKey=True)
        self.assertEqual(policy.save_key(), 'test-save-key')
        self.assertTrue(policy.is_save_key_forced())
        self.assertSetEqual(set(policy.keys()), {
                            'deadline', 'forceSaveKey', 'isPrefixalScope', 'saveKey', 'scope'})
