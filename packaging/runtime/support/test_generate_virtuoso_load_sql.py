import unittest

from generate_virtuoso_load_sql import load_sql_lines


class LoadSqlLinesTest(unittest.TestCase):
    def test_ttl_source_uses_ttlp_mt(self) -> None:
        manifest = {
            "sources": [
                {
                    "path": "/data/sources/demo.ttl",
                    "graph": "http://example.org/graph/demo",
                    "format": "ttl",
                }
            ]
        }

        self.assertEqual(
            load_sql_lines(manifest),
            [
                "DB.DBA.TTLP_MT(file_to_string_output('/data/sources/demo.ttl'), '', 'http://example.org/graph/demo', 0, 0, 0, 0);",
                "checkpoint;",
            ],
        )

    def test_nt_source_uses_ld_dir_and_rdf_loader_run(self) -> None:
        manifest = {
            "sources": [
                {
                    "path": "/data/sources/demo.nt",
                    "graph": "http://example.org/graph/demo",
                    "format": "nt",
                }
            ]
        }

        self.assertEqual(
            load_sql_lines(manifest),
            [
                "ld_dir('/data/sources', 'demo.nt', 'http://example.org/graph/demo');",
                "rdf_loader_run();",
                "checkpoint;",
            ],
        )

    def test_nq_source_uses_null_graph(self) -> None:
        manifest = {
            "sources": [
                {
                    "path": "/data/sources/demo.nq",
                    "graph": None,
                    "format": "nq",
                }
            ]
        }

        self.assertEqual(
            load_sql_lines(manifest),
            [
                "ld_dir('/data/sources', 'demo.nq', NULL);",
                "rdf_loader_run();",
                "checkpoint;",
            ],
        )


if __name__ == "__main__":
    unittest.main()
