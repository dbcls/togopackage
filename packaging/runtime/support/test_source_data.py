import bz2
import gzip
import importlib.util
import lzma
import tempfile
import unittest
from pathlib import Path
import shutil
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from source_data import maybe_decompress, prepare_input_manifest


@unittest.skipUnless(importlib.util.find_spec("yaml"), "PyYAML is required")
class PrepareInputManifestTest(unittest.TestCase):
    def test_source_specific_format_is_written_to_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            config_path = root / "config.yaml"
            data_dir = root / "prepared"

            (root / "sources").mkdir()
            (root / "sources" / "demo.ttl").write_text("@prefix ex: <http://example.org/> .\n", encoding="utf-8")
            (root / "sources" / "demo.nt").write_text(
                "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n",
                encoding="utf-8",
            )
            config_path.write_text(
                "\n".join(
                    [
                        "source:",
                        "  - path: ./sources/demo.ttl",
                        "    graph: http://example.org/graph/demo",
                        "    format: ttl",
                        "  - path: ./sources/demo.nt",
                        "    format: nt",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            manifest = prepare_input_manifest(config_path, data_dir)

            self.assertEqual(len(manifest["sources"]), 2)
            self.assertEqual(manifest["sources"][0]["format"], "ttl")
            self.assertEqual(manifest["sources"][0]["graph"], "http://example.org/graph/demo")
            self.assertEqual(manifest["sources"][1]["format"], "nt")
            self.assertIsNone(manifest["sources"][1]["graph"])
            self.assertNotIn("format", manifest)

    def test_missing_source_format_defaults_to_ttl(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            config_path = root / "config.yaml"
            data_dir = root / "prepared"

            (root / "sources").mkdir()
            (root / "sources" / "demo.ttl").write_text("@prefix ex: <http://example.org/> .\n", encoding="utf-8")
            config_path.write_text(
                "\n".join(
                    [
                        "source:",
                        "  - path: ./sources/demo.ttl",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            manifest = prepare_input_manifest(config_path, data_dir)

            self.assertEqual(manifest["sources"][0]["format"], "ttl")

    def test_nq_source_rejects_graph(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            config_path = root / "config.yaml"
            data_dir = root / "prepared"

            (root / "sources").mkdir()
            (root / "sources" / "demo.nq").write_text(
                "<http://example.org/s> <http://example.org/p> <http://example.org/o> <http://example.org/g> .\n",
                encoding="utf-8",
            )
            config_path.write_text(
                "\n".join(
                    [
                        "source:",
                        "  - path: ./sources/demo.nq",
                        "    format: nq",
                        "    graph: http://example.org/graph/demo",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                ValueError, "source #0 must not specify graph when format is nq"
            ):
                prepare_input_manifest(config_path, data_dir)


class MaybeDecompressTest(unittest.TestCase):
    def test_gzip_is_decompressed(self) -> None:
        self.assertEqual(self._roundtrip(".ttl.gz"), b"demo\n")

    def test_bz2_is_decompressed(self) -> None:
        self.assertEqual(self._roundtrip(".ttl.bz2"), b"demo\n")

    def test_xz_is_decompressed(self) -> None:
        self.assertEqual(self._roundtrip(".ttl.xz"), b"demo\n")

    @unittest.skipUnless(shutil.which("zstd"), "zstd command is required")
    def test_zstd_is_decompressed(self) -> None:
        self.assertEqual(self._roundtrip(".ttl.zst"), b"demo\n")

    def _roundtrip(self, suffix: str) -> bytes:
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / f"source{suffix}"
            if suffix == ".ttl.gz":
                with gzip.open(path, "wb") as handle:
                    handle.write(b"demo\n")
            elif suffix == ".ttl.bz2":
                with bz2.open(path, "wb") as handle:
                    handle.write(b"demo\n")
            elif suffix == ".ttl.xz":
                with lzma.open(path, "wb") as handle:
                    handle.write(b"demo\n")
            else:
                plain = path.with_suffix("")
                plain.write_bytes(b"demo\n")
                try:
                    with path.open("wb") as handle:
                        import subprocess

                        subprocess.run(["zstd", "-q", "-c", str(plain)], check=True, stdout=handle)
                finally:
                    plain.unlink()

            output = maybe_decompress(path)
            return output.read_bytes()


if __name__ == "__main__":
    unittest.main()
