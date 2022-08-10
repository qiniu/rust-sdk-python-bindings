#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from optparse import OptionParser
import qiniu_sdk_alpha
from qiniu_sdk_alpha import upload, upload_token, credential


def main():
    parser = OptionParser()
    parser.add_option('--access-key', dest='access_key',
                      help='Qiniu Access Key')
    parser.add_option('--secret-key', dest='secret_key',
                      help='Qiniu Secret Key')
    parser.add_option('--bucket-name', dest='bucket_name',
                      help='Qiniu Bucket Name')
    parser.add_option('--object-name', dest='object_name',
                      help='Qiniu Object Name')
    parser.add_option('--file', dest='file',
                      help='Upload File Path')
    (options, args) = parser.parse_args()

    if not options.access_key:
        parser.error('--access-key is not given')
    if not options.secret_key:
        parser.error('--secret-key is not given')
    if not options.bucket_name:
        parser.error('--bucket-name is not given')
    if not options.object_name:
        parser.error('--object-name is not given')
    if not options.file:
        parser.error('--file is not given')

    cred = credential.Credential(options.access_key, options.secret_key)
    upload_manager = upload.UploadManager(
        upload.UploadTokenSigner.new_upload_token_provider(upload_token.ObjectUploadTokenProvider(
            options.bucket_name, options.object_name, 3600, cred))
    )
    try:
        upload_manager.form_uploader(upload_progress=on_upload_progress).upload_path(
            options.file, object_name=options.object_name)
    except qiniu_sdk_alpha.QiniuApiCallError as e:
        print('Code: %d, Message: %s, X-Reqid: %s' %
              (e.args[0].status_code, e.args[0].message, e.args[0].x_reqid))


def on_upload_progress(transfer):
    if not transfer.total_bytes:
        print(transfer.transferred_bytes)
    else:
        print('%d / %d => %0.2f%%' % (transfer.transferred_bytes, transfer.total_bytes,
              transfer.transferred_bytes * 100 / transfer.total_bytes))


if __name__ == '__main__':
    main()
