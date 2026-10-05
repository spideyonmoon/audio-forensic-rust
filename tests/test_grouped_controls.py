import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest
import wave

import numpy as np

SPEC = importlib.util.spec_from_file_location('controls', Path(__file__).resolve().parents[1] / 'scripts/build_grouped_controls.py')
C = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(C)


class GroupedControls(unittest.TestCase):
    def test_declared_groups_and_labels(self):
        self.assertEqual(len(C.GROUPS), 6)
        self.assertEqual(len({g[0] for g in C.GROUPS}), 6)
        for split in ['development', 'validation', 'locked_test']:
            self.assertEqual(sorted(g[2] for g in C.GROUPS if g[1] == split), [44100, 48000])
        self.assertEqual(sum(C.label(v, 'aac') == 'present' for v in C.VARIANTS), 3)
        self.assertEqual(sum(C.label(v, 'vorbis') == 'present' for v in C.VARIANTS), 2)
        self.assertEqual(C.label('mp3192_s24', 'aac'), 'absent')
        self.assertEqual(C.label('aac256_trim137_s24', 'vorbis'), 'absent')
        with self.assertRaises(ValueError):
            C.label('unknown-recipe', 'aac')

    def test_native_pcm_encoding_matches_integer_definition(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'native.wav'
            words = np.array([[-8388608, -1], [0, 1], [8388607, 256]], dtype='<i4')
            expected = b''.join(struct.pack('<i', int(x) * 256) for x in words.flat)
            self.assertEqual(C.write_wav(path, words, 48000), hashlib.sha256(expected).hexdigest())
            with wave.open(str(path), 'rb') as reader:
                self.assertEqual((reader.getnchannels(), reader.getsampwidth(), reader.getnframes()), (2, 3, 3))
                raw = reader.readframes(3)
            decoded = [int.from_bytes(raw[i:i+3], 'little', signed=True) for i in range(0, len(raw), 3)]
            self.assertEqual(decoded, list(words.flat))

    def test_families_repeat_exactly_and_remain_distinct(self):
        hashes = set()
        for family, _, rate, seed in C.GROUPS:
            first = C.generated_words(family, rate, seed)
            second = C.generated_words(family, rate, seed)
            self.assertTrue(np.array_equal(first, second))
            self.assertEqual(first.shape, (rate * C.DURATION, 2))
            self.assertLess(np.max(np.abs(first)), 2**23)
            self.assertFalse(np.array_equal(first[:, 0], first[:, 1]))
            hashes.add(hashlib.sha256(first.tobytes()).hexdigest())
        self.assertEqual(len(hashes), 6)

    def test_inventory_mutation_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'source'
            source.write_bytes(b'original')
            C.save(root / 'inventory.json', {'sha256': {'source': C.sha(source)}})
            C.verify(root)
            source.write_bytes(b'changed')
            with self.assertRaises(ValueError):
                C.verify(root)
            with self.assertRaises(FileExistsError):
                C.save(root / 'inventory.json', {})


if __name__ == '__main__':
    unittest.main()
