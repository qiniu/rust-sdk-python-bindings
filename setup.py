from setuptools import setup
from setuptools_rust import Binding, RustExtension
from setuptools.dist import Distribution
from os import path

__dir__ = path.abspath(path.dirname(__file__))
readme_path = path.join(__dir__, 'README.md')

with open(readme_path, encoding='utf-8') as readme:
    long_description = readme.read()

setup(
    name='qiniu-sdk-python-bindings',
    version='0.1.1',
    author='Rong Zhou',
    author_email='zhourong@qiniu.com',
    keywords='qiniu storage rust python',
    license='MIT',
    packages=['qiniu_sdk_python_bindings'],
    description='Qiniu Rust SDK Bindings to Python',
    long_description=long_description,
    long_description_content_type='text/markdown',
    zip_safe=False,
    rust_extensions=[RustExtension("qiniu_sdk_python_bindings.qiniu_sdk_python_bindings", binding=Binding.PyO3, debug=False)],
    platforms='any',
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "License :: OSI Approved :: MIT License",
    ],
)
