#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from optparse import OptionParser
import qiniu_sdk_alpha
from qiniu_sdk_alpha import objects, credential


def main():
    parser = OptionParser()
    parser.add_option('--access-key', dest='access_key',
                      help='Qiniu Access Key')
    parser.add_option('--secret-key', dest='secret_key',
                      help='Qiniu Secret Key')
    parser.add_option('--from-bucket-name', dest='from_bucket_name',
                      help='From Qiniu Bucket Name')
    parser.add_option('--from-object-name', dest='from_object_name',
                      help='From Qiniu Object Name')
    parser.add_option('--to-bucket-name', dest='to_bucket_name',
                      help='To Qiniu Bucket Name')
    parser.add_option('--to-object-name', dest='to_object_name',
                      help='To Qiniu Object Name')
    (options, args) = parser.parse_args()

    if not options.access_key:
        parser.error('--access-key is not given')
    if not options.secret_key:
        parser.error('--secret-key is not given')
    if not options.from_bucket_name:
        parser.error('--from-bucket-name is not given')
    if not options.from_object_name:
        parser.error('--from-object-name is not given')
    if not options.to_bucket_name:
        parser.error('--to-bucket-name is not given')
    if not options.to_object_name:
        parser.error('--to-object-name is not given')

    cred = credential.Credential(options.access_key, options.secret_key)
    objects_manager = objects.ObjectsManager(cred)
    bucket = objects_manager.bucket(options.from_bucket_name)
    try:
        bucket.copy_object_to(
            options.from_object_name, options.to_bucket_name, options.to_object_name).call()
    except qiniu_sdk_alpha.QiniuApiCallError as e:
        print('Code: %d, Message: %s, X-Reqid: %s' %
              (e.args[0].status_code, e.args[0].message, e.args[0].x_reqid))


if __name__ == '__main__':
    main()
