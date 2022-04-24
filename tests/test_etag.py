import qiniu_rust_bindings
import unittest
import io

class TestEtag(unittest.TestCase):
    def test_empty_etag_of(self):
        empty = io.StringIO()
        self.assertEqual(qiniu_rust_bindings.etag_of(empty), 'Fto5o-5ea0sNMlW_75VgGJCv2AcJ')
    def test_simple_etag_of(self):
        simple = io.StringIO()
        simple.write('etag')
        simple.seek(0, io.SEEK_SET)
        self.assertEqual(qiniu_rust_bindings.etag_of(simple), 'FpLiADEaVoALPkdb8tJEJyRTXoe_')

if __name__ == '__main__':
    unittest.main()
