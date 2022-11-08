# Qiniu Resource Storage Binding SDK for Python

[![Run Test Cases](https://github.com/qiniu/rust-sdk-python-bindings/actions/workflows/ci-test.yml/badge.svg)](https://github.com/qiniu/rust-sdk-python-bindings/actions/workflows/ci-test.yml)
[![GitHub release](https://img.shields.io/github/v/tag/qiniu/rust-sdk-python-bindings.svg?label=release)](https://github.com/qiniu/rust-sdk-python-bindings/releases)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/qiniu/rust-sdk-python-bindings/blob/master/LICENSE)

## 概要

Qiniu SDK for Python 包含以下特性：

- 通过提供多个不同的 Module，为不同层次的开发都提供了方便易用的编程接口。
- 同时提供阻塞 IO 接口和基于 Async/Await 的异步 IO 接口。
- 用 PyO3 封装 Rust 代码，因此安装该插件需要您先安装最新版本的 Rust（安装方式请访问 [rustup.rs](https://rustup.rs/)）。

## 安装步骤

1. 确认 Rust 已经被安装，且版本号大于 `1.62.0`（可以用 `rustc --version` 查看）
2. 确认 Python 已经被安装，且版本号大于 `3.8.0`（可以用 `python3 --version` 查看）
3. 仅对于中国大陆用户，可以参考清华大学开源软件镜像站的 [Rust crates.io 索引镜像使用帮助](https://mirrors.tuna.tsinghua.edu.cn/help/crates.io-index.git/) 和 [PyPI 镜像使用帮助](https://mirrors.tuna.tsinghua.edu.cn/help/pypi/) 加速相关依赖的下载速度
4. 执行 `pip3 install qiniu-bindings`

## 代码示例

### 客户端上传凭证

客户端（移动端或者Web端）上传文件的时候，需要从客户自己的业务服务器获取上传凭证，而这些上传凭证是通过服务端的 SDK 来生成的，然后通过客户自己的业务API分发给客户端使用。根据上传的业务需求不同，七牛云 Python SDK 支持丰富的上传凭证生成方式。

#### 简单上传的凭证

最简单的上传凭证只需要 `access key`，`secret key` 和 `bucket` 就可以。

```python
from qiniu_bindings import upload_token, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
upload_token = upload_token.UploadPolicy.new_for_bucket(
    bucket_name, 3600).build().to_upload_token_provider(cred)
print(upload_token)
```

#### 覆盖上传的凭证

覆盖上传除了需要简单上传所需要的信息之外，还需要想进行覆盖的对象名称 `object name`，这个对象名称同时是客户端上传代码中指定的对象名称，两者必须一致。

```python
from qiniu_bindings import upload_token, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
upload_token = upload_token.UploadPolicy.new_for_object(
    bucket_name, object_name, 3600).build().to_upload_token_provider(cred)
print(upload_token)

#### 自定义上传回复的凭证

默认情况下，文件上传到七牛之后，在没有设置 `returnBody` 或者回调相关的参数情况下，七牛返回给上传端的回复格式为 `hash` 和 `key`，例如：

```json
{"hash":"Ftgm-CkWePC9fzMBTRNmPMhGBcSV","key":"qiniu.jpg"}
```

有时候我们希望能自定义这个返回的 JSON 格式的内容，可以通过设置 `returnBody` 参数来实现，在 `returnBody` 中，我们可以使用七牛支持的魔法变量和自定义变量。

```python
from qiniu_bindings import upload_token, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
upload_token = upload_token.UploadPolicy.new_for_object(
    bucket_name, object_name, 3600,
    returnBody='{"key":"$(key)","hash":"$(etag)","bucket":"$(bucket)","fsize":$(fsize)}'
).build().to_upload_token_provider(cred)
print(upload_token)
```

则文件上传到七牛之后，收到的回复内容如下：

```json
{"key":"qiniu.jpg","hash":"Ftgm-CkWePC9fzMBTRNmPMhGBcSV","bucket":"if-bc","fsize":39335}
```

#### 带回调业务服务器的凭证

上面生成的自定义上传回复的上传凭证适用于上传端（无论是客户端还是服务端）和七牛服务器之间进行直接交互的情况下。在客户端上传的场景之下，有时候客户端需要在文件上传到七牛之后，从业务服务器获取相关的信息，这个时候就要用到七牛的上传回调及相关回调参数的设置。

```python
from qiniu_bindings import upload_token, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
builder = upload_token.UploadPolicy.new_for_object(bucket_name, object_name, 3600)
builder.callback(['http://api.example.com/qiniu/upload/callback'], body='{"key":"$(key)","hash":"$(etag)","bucket":"$(bucket)","fsize":$(fsize)}', body_type='application/json')
upload_token = builder.build().to_upload_token_provider(cred)
print(upload_token)
```

在使用了上传回调的情况下，客户端收到的回复就是业务服务器响应七牛的JSON格式内容。
通常情况下，我们建议使用 `application/json` 格式来设置 `callback_body`，保持数据格式的统一性。实际情况下，`callback_body` 也支持 `application/x-www-form-urlencoded` 格式来组织内容，这个主要看业务服务器在接收到 `callback_body` 的内容时如何解析。例如：

```python
from qiniu_bindings import upload_token, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
builder = upload_token.UploadPolicy.new_for_object(bucket_name, object_name, 3600)
builder.callback(['http://api.example.com/qiniu/upload/callback'], body='key=$(key)&hash=$(etag)&bucket=$(bucket)&fsize=$(fsize)')
upload_token = builder.build().to_upload_token_provider(cred)
print(upload_token)
```

### 服务端直传

服务端直传是指客户利用七牛服务端 SDK 从服务端直接上传文件到七牛云，交互的双方一般都在机房里面，所以服务端可以自己生成上传凭证，然后利用 SDK 中的上传逻辑进行上传，最后从七牛云获取上传的结果，这个过程中由于双方都是业务服务器，所以很少利用到上传回调的功能，而是直接自定义 `returnBody` 来获取自定义的回复内容。

#### 文件上传

最简单的就是上传本地文件，直接指定文件的完整路径即可上传。

```python
from qiniu_bindings import upload, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
upload_manager = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(cred, bucket_name, 3600))
uploader = upload_manager.auto_uploader()
uploader.upload_path('/home/qiniu/test.png', object_name=object_name, file_name=object_name)
```

在这个场景下，`AutoUploader` 会自动根据文件尺寸判定是否启用断点续上传，如果文件较大，上传了一部分时因各种原因从而中断，再重新执行相同的代码时，SDK 会尝试找到先前没有完成的上传任务，从而继续进行上传。

#### 字节数组上传 / 数据流上传

可以支持将内存中的字节数组或实现了 `read` 方法的实例上传到空间中。

```python
from qiniu_bindings import upload, credential
import io

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
upload_manager = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(cred, bucket_name, 3600))
uploader = upload_manager.auto_uploader()
uploader.upload_reader(io.BytesIO(b'hello qiniu cloud'), object_name=object_name, file_name=object_name)
```

#### 自定义参数上传

```python
from qiniu_bindings import upload, credential

def on_policy_generated(builder):
    builder.return_body = '{"key":"$(key)","hash":"$(etag)","fname":"$(x:fname)","age":$(x:age)}'

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
upload_manager = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(cred, bucket_name, 3600, on_policy_generated=on_policy_generated))
uploader = upload_manager.auto_uploader()
uploader.upload_path('/home/qiniu/test.png', object_name=object_name, file_name=object_name, custom_vars={'fname': '123.jpg', 'age': '20'})
```

#### 私有云上传

```python
from qiniu_bindings import upload, credential, http_client

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
upload_manager = upload.UploadManager(upload.UploadTokenSigner.new_credential_provider(cred, bucket_name, 3600), uc_endpoints=http_client.Endpoints(['ucpub-qos.pocdemo.qiniu.io']))
uploader = upload_manager.auto_uploader()
uploader.upload_path('/home/qiniu/test.png', object_name=object_name, file_name=object_name)
```

### 下载文件

文件下载分为公开空间的文件下载和私有空间的文件下载。

#### 公开空间

```python
from qiniu_bindings import download

object_name = '公司/存储/qiniu.jpg'
domain = 'devtools.qiniu.com'
path = '/home/user/qiniu.jpg'
download_manager = download.DownloadManager(download.StaticDomainsUrlsGenerator([domain], use_https=False)) # 设置为 HTTP 协议
download_manager.download_to_path(object_name, path)
```

#### 私有空间

```python
from qiniu_bindings import download, credential

object_name = '公司/存储/qiniu.jpg'
domain = 'devtools.qiniu.com'
path = '/home/user/qiniu.jpg'
access_key = 'access key'
secret_key = 'secret key'
cred = credential.Credential(access_key, secret_key)
download_manager = download.DownloadManager(download.UrlsSigner(cred, download.StaticDomainsUrlsGenerator([domain], use_https=False))) # 设置为 HTTP 协议
download_manager.download_to_path(object_name, path)
```

### 资源管理

#### 获取文件信息

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
response = bucket.stat_object(object_name).call()
print(response['hash'])
print(response['fsize'])
print(response['mimeType'])
print(response['putTime'])
```

#### 修改文件类型

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
bucket.modify_object_metadata(object_name, 'application/json').call()
```

#### 移动或重命名文件

移动操作本身支持移动文件到相同，不同空间中，在移动的同时也可以支持文件重命名。唯一的限制条件是，移动的源空间和目标空间必须在同一个机房。

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
to_bucket_name = 'to bucket name"
to_object_name = "new object name"
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
bucket.move_object_to(object_name, to_bucket_name, to_object_name).call()
```

#### 复制文件副本

文件的复制和文件移动其实操作一样，主要的区别是移动后源文件不存在了，而复制的结果是源文件还存在，只是多了一个新的文件副本。

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
to_bucket_name = "to bucket name"
to_object_name = "new object name"
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
bucket.copy_object_to(object_name, to_bucket_name, to_object_name).call()
```

#### 删除空间中的文件

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
bucket.delete_object(object_name).call()
```
#### 设置或更新文件的生存时间

可以给已经存在于空间中的文件设置文件生存时间，或者更新已设置了生存时间但尚未被删除的文件的新的生存时间。

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
object_name = 'object name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
bucket.modify_object_life_cycle(object_name, delete_after_days=10).call()
```

#### 获取空间文件列表

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for obj in bucket.list():
    print('%s\n  hash: %s\n  size: %d\n  mime type: %s' % (obj['key'], obj['hash'], obj['fsize'], obj['mimeType']))
```

#### 私有云中获取空间文件列表

```python
from qiniu_bindings import objects, credential, http_client

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred, uc_endpoints=http_client.Endpoints(['ucpub-qos.pocdemo.qiniu.io']), use_https=False).bucket(bucket_name) # 私有云普遍使用 HTTP 协议，而 SDK 则默认为 HTTPS 协议
for obj in bucket.list():
    print('%s\n  hash: %s\n  size: %d\n  mime type: %s' % (obj['key'], obj['hash'], obj['fsize'], obj['mimeType']))
```

### 资源管理批量操作

#### 批量获取文件信息

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for result in bucket.batch_ops([
    bucket.stat_object('qiniu.jpg'),
    bucket.stat_object('qiniu.mp4'),
    bucket.stat_object('qiniu.png'),
]):
    if result.error:
        print('error: %s' % result.error)
    else:
        print('hash: %s\nsize: %d\nmime type: %s' % (result.data['hash'], result.data['fsize'], result.data['mimeType']))
```

#### 批量修改文件类型

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for result in bucket.batch_ops([
    bucket.modify_object_metadata('qiniu.jpg', 'image/jpeg'),
    bucket.modify_object_metadata('qiniu.mp4', 'image/png'),
    bucket.modify_object_metadata('qiniu.png', 'video/mp4'),
]):
    if result.error:
        print('error: %s' % result.error)
    else:
        print('ok')
```

#### 批量删除文件

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for result in bucket.batch_ops([
    bucket.delete_object('qiniu.jpg'),
    bucket.delete_object('qiniu.mp4'),
    bucket.delete_object('qiniu.png'),
]):
    if result.error:
        print('error: %s' % result.error)
    else:
        print('ok')
```

#### 批量移动或重命名文件

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for result in bucket.batch_ops([
    bucket.move_object_to('qiniu.jpg', bucket_name, 'qiniu.jpg.move'),
    bucket.move_object_to('qiniu.mp4', bucket_name, 'qiniu.mp4.move'),
    bucket.move_object_to('qiniu.png', bucket_name, 'qiniu.png.move'),
]):
    if result.error:
        print('error: %s' % result.error)
    else:
        print('ok')
```

#### 批量复制文件

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for result in bucket.batch_ops([
    bucket.copy_object_to('qiniu.jpg', bucket_name, 'qiniu.jpg.move'),
    bucket.copy_object_to('qiniu.mp4', bucket_name, 'qiniu.mp4.move'),
    bucket.copy_object_to('qiniu.png', bucket_name, 'qiniu.png.move'),
]):
    if result.error:
        print('error: %s' % result.error)
    else:
        print('ok')
```

#### 批量解冻归档存储类型文件

```python
from qiniu_bindings import objects, credential

access_key = 'access key'
secret_key = 'secret key'
bucket_name = 'bucket name'
cred = credential.Credential(access_key, secret_key)
bucket = objects.ObjectsManager(cred).bucket(bucket_name)
for result in bucket.batch_ops([
    bucket.restore_archived_object('qiniu.jpg', 7),
    bucket.restore_archived_object('qiniu.mp4', 7),
    bucket.restore_archived_object('qiniu.png', 7),
]):
    if result.error:
        print('error: %s' % result.error)
    else:
        print('ok')
```

## 最低支持的 Python 版本（MSPV）

- 3.8.0

## 最低支持的 Rust 版本（MSRV）

- 1.62.0

## 编码规范

- 通过 `cargo clippy` 检查，并经过 `rustfmt` 格式化。
- 所有公开接口都需要文档注释。
- 所有阻塞操作都提供异步无阻塞版本。
- 尽可能保证仅使用安全的代码。

## 联系我们

- 如果需要帮助，请提交工单（在portal右侧点击咨询和建议提交工单，或者直接向 support@qiniu.com 发送邮件）
- 如果有什么问题，可以到问答社区提问，[问答社区](http://qiniu.segmentfault.com/)
- 更详细的文档，见[官方文档站](http://developer.qiniu.com/)
- 如果发现了bug， 欢迎提交 [Issue](https://github.com/qiniu/rust-sdk/issues)
- 如果有功能需求，欢迎提交 [Issue](https://github.com/qiniu/rust-sdk/issues)
- 如果要提交代码，欢迎提交 [Pull Request](https://github.com/qiniu/rust-sdk/pulls)
- 欢迎关注我们的[微信](https://www.qiniu.com/contact) [微博](http://weibo.com/qiniutek)，及时获取动态信息。

## 代码许可

This project is licensed under the [MIT license].

[MIT license]: https://github.com/qiniu/rust-sdk/blob/master/LICENSE
