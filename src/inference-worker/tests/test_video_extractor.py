import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from media.video_extractor import extract_video_frames, is_video_file


class VideoExtensionTests(unittest.TestCase):
    def test_required_video_containers_are_supported(self):
        for extension in ("mkv", "avi", "mp4", "mov"):
            with self.subTest(extension=extension):
                self.assertTrue(is_video_file(f"sample.{extension}"))
                self.assertTrue(is_video_file(f"sample.{extension.upper()}"))


class VideoDecodeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.ffmpeg = shutil.which("ffmpeg")
        if not cls.ffmpeg:
            raise unittest.SkipTest("ffmpeg is required for video integration tests")
        cls.temp_dir = tempfile.TemporaryDirectory(prefix="framescout-video-tests-")
        root = Path(cls.temp_dir.name)
        cls.samples = {}
        codecs = {
            "mkv": ["-c:v", "libx264"],
            "avi": ["-c:v", "mpeg4"],
            "mp4": ["-c:v", "libx264", "-pix_fmt", "yuv420p"],
            "mov": ["-c:v", "libx264", "-pix_fmt", "yuv420p"],
        }
        for extension, codec_args in codecs.items():
            path = root / f"sample.{extension}"
            subprocess.run(
                [
                    cls.ffmpeg,
                    "-y",
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=320x180:rate=12",
                    "-t",
                    "2",
                    *codec_args,
                    str(path),
                ],
                check=True,
            )
            cls.samples[extension] = path

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, "temp_dir"):
            cls.temp_dir.cleanup()

    def test_extracts_required_containers(self):
        for extension, path in self.samples.items():
            with self.subTest(extension=extension):
                frames = list(extract_video_frames(str(path), max_duration=2, max_frames=4))
                self.assertGreater(len(frames), 0)
                self.assertTrue(
                    all(frame.ndim == 3 and frame.shape[2] == 3 for _, frame in frames)
                )

    def test_ffmpeg_fallback_when_opencv_cannot_open(self):
        class ClosedCapture:
            def isOpened(self):
                return False

            def release(self):
                pass

        sample = self.samples["mkv"]
        with mock.patch(
            "media.video_extractor.cv2.VideoCapture", return_value=ClosedCapture()
        ):
            frames = list(extract_video_frames(str(sample), max_duration=2, max_frames=4))
        self.assertGreater(len(frames), 0)
        self.assertEqual(frames[0][1].shape[2], 3)


if __name__ == "__main__":
    unittest.main()
