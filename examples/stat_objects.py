#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from optparse import OptionParser
import sys
from qiniu_sdk_alpha import objects, credential


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
    ops = []
    for line in sys.stdin:
        ops.append(bucket.stat_object(line.strip()))
    for result in bucket.batch_ops(ops):
        if result.error:
            print('Code: %d, Message: %s' %
                  (result.error.args[0].status_code, result.error.args[0].message))
        else:
            print(result.data)


if __name__ == '__main__':
    main()
