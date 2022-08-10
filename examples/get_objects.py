#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from optparse import OptionParser
import qiniu_sdk_alpha
from qiniu_sdk_alpha import credential, objects


def main():
    parser = OptionParser()
    parser.add_option('--access-key', dest='access_key',
                      help='Qiniu Access Key')
    parser.add_option('--secret-key', dest='secret_key',
                      help='Qiniu Secret Key')
    parser.add_option('--bucket-name', dest='bucket_name',
                      help='Qiniu Bucket Name')
    (options, args) = parser.parse_args()

    if not options.access_key:
        parser.error('--access-key is not given')
    if not options.secret_key:
        parser.error('--secret-key is not given')
    if not options.bucket_name:
        parser.error('--bucket-name is not given')

    cred = credential.Credential(options.access_key, options.secret_key)
    objects_manager = objects.ObjectsManager(cred)
    bucket = objects_manager.bucket(options.bucket_name)
    try:
        for object in bucket:
            print('%s\n  hash: %s\n  size: %d\n  mime type: %s' % (
                object['key'], object['hash'], object['fsize'], object['mimeType']))
    except qiniu_sdk_alpha.QiniuApiCallError as e:
        print('Code: %d, Message: %s, X-Reqid: %s' %
              (e.args[0].status_code, e.args[0].message, e.args[0].x_reqid))


if __name__ == '__main__':
    main()
