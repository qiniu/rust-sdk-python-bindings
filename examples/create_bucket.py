#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from optparse import OptionParser
import qiniu_bindings
from qiniu_bindings import credential, http_client, apis


def main():
    parser = OptionParser()
    parser.add_option('--access-key', dest='access_key',
                      help='Qiniu Access Key')
    parser.add_option('--secret-key', dest='secret_key',
                      help='Qiniu Secret Key')
    parser.add_option('--bucket-name', dest='bucket_name',
                      help='Qiniu Bucket Name')
    parser.add_option('--region-id', dest='region_id',
                      help='Qiniu Region ID')
    (options, args) = parser.parse_args()

    if not options.access_key:
        parser.error('--access-key is not given')
    if not options.secret_key:
        parser.error('--secret-key is not given')
    if not options.bucket_name:
        parser.error('--bucket-name is not given')
    if not options.region_id:
        parser.error('--region-id is not given')

    cred = credential.Credential(options.access_key, options.secret_key)
    region = http_client.AllRegionsProvider(cred).get()
    try:
        apis.storage.create_bucket.Client().call(http_client.EndpointsProvider(region), cred,
                                                 bucket=options.bucket_name, region=options.region_id)
    except qiniu_bindings.QiniuApiCallError as e:
        print('Code: %d, Message: %s, X-Reqid: %s' %
              (e.args[0].status_code, e.args[0].message, e.args[0].x_reqid))


if __name__ == '__main__':
    main()
