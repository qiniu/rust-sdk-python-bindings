#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from optparse import OptionParser
import sys
import qiniu_sdk_bindings
from qiniu_sdk_bindings import objects, credential


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
    batch_ops = bucket.batch_ops()
    for line in sys.stdin:
        batch_ops.add_operation(bucket.stat_object(line.strip()))
    it = iter(batch_ops)
    while True:
        try:
            object = next(it)
            print(object)
        except qiniu_sdk_bindings.QiniuApiCallError as e:
            print('Code: %d, Message: %s' %
                  (e.args[0].status_code, e.args[0].message))
        except StopIteration:
            break


if __name__ == '__main__':
    main()
