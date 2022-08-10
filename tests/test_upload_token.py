from qiniu_sdk_alpha import upload_token, credential
import unittest


class TestUploadPolicy(unittest.TestCase):
    def test_bucket_policy(self):
        policy = upload_token.UploadPolicy.new_for_bucket(
            'test-bucket', 3600).build()
        self.assertEqual(policy.bucket, 'test-bucket')

    def test_object_policy(self):
        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600).build()
        self.assertEqual(policy.bucket, 'test-bucket')
        self.assertEqual(policy.key, 'test-object')
        self.assertFalse(policy.use_prefixal_object_key)

    def test_object_prefix_policy(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600).build()
        self.assertEqual(policy.bucket, 'test-bucket')
        self.assertEqual(policy.key, 'test-object')
        self.assertTrue(policy.use_prefixal_object_key)

    def test_json_convertion(self):
        policy = upload_token.UploadPolicy.new_for_objects_with_prefix(
            'test-bucket', 'test-object', 3600).build()
        new_policy = upload_token.UploadPolicy.from_json(policy.as_json())
        self.assertEqual(new_policy.bucket, 'test-bucket')
        self.assertEqual(new_policy.key, 'test-object')
        self.assertTrue(new_policy.use_prefixal_object_key)

    def test_insert_only(self):
        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.insert_only()
        policy = builder.build()
        self.assertTrue(policy.is_insert_only)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, insertOnly=1).build()
        self.assertTrue(policy.is_insert_only)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, insertOnly=0).build()
        self.assertFalse(policy.is_insert_only)

    def test_mime_detection(self):
        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.enable_mime_detection()
        policy = builder.build()
        self.assertTrue(policy.mime_detection_enabled)

        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.disable_mime_detection()
        policy = builder.build()
        self.assertFalse(policy.mime_detection_enabled)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, detectMime=1).build()
        self.assertTrue(policy.mime_detection_enabled)
        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, detectMime=0).build()
        self.assertFalse(policy.mime_detection_enabled)

    def test_return_url(self):
        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.return_url = 'http://www.qiniu.com'
        policy = builder.build()
        self.assertEqual(policy.return_url, 'http://www.qiniu.com')

        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.return_url = 'http://www.qiniu.com'
        builder.return_body = '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}'
        policy = builder.build()
        self.assertEqual(policy.return_url, 'http://www.qiniu.com')
        self.assertEqual(
            policy.return_body, '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, returnUrl='http://www.qiniu.com').build()
        self.assertEqual(policy.return_url, 'http://www.qiniu.com')

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600,
            returnUrl='http://www.qiniu.com',
            returnBody='{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}').build()
        self.assertEqual(policy.return_url, 'http://www.qiniu.com')
        self.assertEqual(
            policy.return_body, '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')

    def test_callback_urls(self):
        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.callback(['http://cb1.com', 'http://cb2.com'], '',
                         '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')
        policy = builder.build()
        self.assertEqual(policy.callback_urls, [
            'http://cb1.com', 'http://cb2.com'])
        self.assertEqual(
            policy.callback_body, '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600,
            callbackUrl='http://cb1.com;http://cb2.com',
            callbackBody='{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}').build()
        self.assertEqual(policy.callback_urls, [
            'http://cb1.com', 'http://cb2.com'])
        self.assertEqual(
            policy.callback_body, '{"key":$(key),"hash":$(etag),"w":$(imageInfo.width),"h":$(imageInfo.height)}')

    def test_save_key(self):
        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.save_as('test-save-key')
        policy = builder.build()
        self.assertEqual(policy.save_key, 'test-save-key')
        self.assertFalse(policy.is_save_key_forced)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600,
            saveKey='test-save-key', forceSaveKey=False).build()
        self.assertEqual(policy.save_key, 'test-save-key')
        self.assertFalse(policy.is_save_key_forced)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600,
            saveKey='test-save-key', forceSaveKey=True).build()
        self.assertEqual(policy.save_key, 'test-save-key')
        self.assertTrue(policy.is_save_key_forced)
        self.assertSetEqual(set(policy.keys), {
                            'deadline', 'forceSaveKey', 'saveKey', 'scope'})

    def test_file_size_limitation(self):
        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.file_size_limitation(None, 5)
        policy = builder.build()
        self.assertEqual(policy.minimum_file_size, None)
        self.assertEqual(policy.maximum_file_size, 5)

        builder = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600)
        builder.file_size_limitation(5)
        policy = builder.build()
        self.assertEqual(policy.minimum_file_size, 5)
        self.assertEqual(policy.maximum_file_size, None)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, fsizeMin=5).build()
        self.assertEqual(policy.minimum_file_size, 5)
        self.assertEqual(policy.maximum_file_size, None)

        policy = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600, fsizeLimit=5).build()
        self.assertEqual(policy.minimum_file_size, None)
        self.assertEqual(policy.maximum_file_size, 5)


class TestUploadTokenProvider(unittest.TestCase):
    def test_static_upload_token_provider(self):
        cred = credential.Credential('test-ak', 'test-sk')
        provider = upload_token.UploadPolicy.new_for_object(
            'test-bucket', 'test-object', 3600).build().to_upload_token_provider(cred)
        token = provider.to_token_string()
        self.assertTrue(token.startswith('test-ak:'))
        provider = upload_token.StaticUploadTokenProvider(token)
        self.assertEqual(provider.access_key(), 'test-ak')
        self.assertEqual(provider.bucket_name(), 'test-bucket')
        self.assertEqual(provider.policy().key, 'test-object')

    def test_bucket_upload_token_provider(self):
        cred = credential.Credential('test-ak', 'test-sk')
        provider = upload_token.BucketUploadTokenProvider(
            'test-bucket', 3600, cred)
        self.assertTrue(provider.to_token_string().startswith('test-ak:'))
        self.assertEqual(provider.bucket_name(), 'test-bucket')
        self.assertEqual(provider.policy().key, None)

        def on_policy_generated(builder):
            builder.insert_only()
            builder.enable_mime_detection()
            builder.return_url = 'http://abc.com'
            return builder

        provider = upload_token.BucketUploadTokenProvider(
            'test-bucket', 3600, cred, on_policy_generated=on_policy_generated)
        self.assertTrue(provider.policy().is_insert_only)
        self.assertTrue(provider.policy().mime_detection_enabled)
        self.assertEqual(provider.policy().return_url, 'http://abc.com')
        self.assertTrue(provider.to_token_string().startswith('test-ak:'))

    def test_object_upload_token_provider(self):
        cred = credential.Credential('test-ak', 'test-sk')
        provider = upload_token.ObjectUploadTokenProvider(
            'test-bucket', 'test-object', 3600, cred)
        self.assertTrue(provider.to_token_string().startswith('test-ak:'))
        self.assertEqual(provider.bucket_name(), 'test-bucket')
        self.assertEqual(provider.policy().key, 'test-object')

        def on_policy_generated(builder):
            builder.insert_only()
            builder.enable_mime_detection()
            builder.return_url = 'http://abc.com'
            return builder

        provider = upload_token.ObjectUploadTokenProvider(
            'test-bucket', 'test-object', 3600, cred, on_policy_generated=on_policy_generated)
        self.assertTrue(provider.policy().is_insert_only)
        self.assertTrue(provider.policy().mime_detection_enabled)
        self.assertEqual(provider.policy().return_url, 'http://abc.com')
        self.assertTrue(provider.to_token_string().startswith('test-ak:'))


if __name__ == '__main__':
    unittest.main()
