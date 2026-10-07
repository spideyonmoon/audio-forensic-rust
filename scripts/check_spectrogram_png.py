"""Independent PNG decode and frozen P06 data/report preservation, generated audio only."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zlib

import jsonschema

ROOT = Path(__file__).resolve().parents[1]


def decode_png(path):
    """PNG RGB8, chunk CRCs and all five row filters, without the Rust encoder/decoder."""
    encoded = path.read_bytes()
    assert encoded[:8] == b'\x89PNG\r\n\x1a\n'
    at, compressed, caption, dpi, srgb = 8, bytearray(), None, None, None
    while at < len(encoded):
        size = struct.unpack_from('>I', encoded, at)[0]
        kind = encoded[at + 4:at + 8]
        payload = encoded[at + 8:at + 8 + size]
        crc = struct.unpack_from('>I', encoded, at + 8 + size)[0]
        assert zlib.crc32(kind + payload) == crc, kind
        at += size + 12
        if kind == b'IHDR':
            width, height, depth, color, comp, filt, interlace = struct.unpack('>IIBBBBB', payload)
            assert (depth, color, comp, filt, interlace) == (8, 2, 0, 0, 0)
        elif kind == b'IDAT':
            compressed.extend(payload)
        elif kind == b'pHYs':
            dpi = struct.unpack('>IIB', payload)
        elif kind == b'sRGB':
            srgb = payload
        elif kind == b'iTXt':
            key, rest = payload.split(b'\0', 1)
            flag, method = rest[:2]
            language, translated, content = rest[2:].split(b'\0', 2)
            assert method == 0 and flag in (0, 1)
            if key == b'Alfred canvas':
                caption = json.loads(zlib.decompress(content) if flag else content)
        elif kind == b'IEND':
            assert at == len(encoded)
            break
    assert dpi == (11811, 11811, 1) and srgb == b'\0' and caption
    raw = zlib.decompress(compressed)
    stride = width * 3
    assert len(raw) == height * (stride + 1)
    pixels, previous = bytearray(), bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind = raw[start]
        row = bytearray(raw[start + 1:start + 1 + stride])
        assert 0 <= kind <= 4
        for x in range(stride):
            left = row[x - 3] if x >= 3 else 0
            up = previous[x]
            upper_left = previous[x - 3] if x >= 3 else 0
            if kind == 1:
                predictor = left
            elif kind == 2:
                predictor = up
            elif kind == 3:
                predictor = (left + up) // 2
            elif kind == 4:
                p = left + up - upper_left
                a, b, c = abs(p - left), abs(p - up), abs(p - upper_left)
                predictor = left if a <= b and a <= c else (up if b <= c else upper_left)
            else:
                predictor = 0
            row[x] = (row[x] + predictor) & 255
        pixels.extend(row)
        previous = row
    return width, height, pixels, caption


def check(binary, controls, output):
    output.mkdir(parents=True, exist_ok=False)
    native = jsonschema.Draft202012Validator(json.loads((ROOT / 'schemas/analysis-report-0.18.0.schema.json').read_text()))
    artifact = jsonschema.Draft202012Validator(json.loads((ROOT / 'schemas/spectrogram-1.schema.json').read_text()))
    sizes = {'standard': (1600, 900), 'publication': (2560, 1440), 'large': (3840, 2160)}
    cases = [('tone8k', None, 'publication'), ('dual48k', None, 'standard'),
             ('silent', None, 'standard'), ('anti', None, 'standard'),
             ('burst', 1., 'standard'), ('long384k', None, 'large')]
    receipts, legacy_caption_repairs = [], 0
    for name, prefix, size in cases:
        stem = name + ('-prefix' if prefix is not None else '')
        source, path = controls / (name + '.wav'), output / (stem + '.png')
        process = subprocess.run([str(binary), str(source), 'full' if prefix is None else str(prefix), str(path), size], capture_output=True, text=True, encoding='utf-8', timeout=180)
        assert process.returncode == 0, process.stderr[-2000:]
        p = json.loads(process.stdout)
        export = json.loads(process.stderr)
        assert export['status'] == 'available' and Path(export['path']) == path
        r, a, presentation = p['measurement'], p['spectrogram'], p['presentation']
        native.validate(r)
        artifact.validate(a)
        assert r['ancestry_verdict'] == 'INCONCLUSIVE' and r['evidence_index'] is None
        baseline = json.loads((controls / (stem + '.json')).read_text(encoding='utf-8'))
        # Paths are display names only; compare every frozen numerical/native field.
        expected, actual = copy.deepcopy(baseline['measurement']), copy.deepcopy(r)
        for key in ['engine_version', 'source']:
            expected.pop(key)
            actual.pop(key)
        # The original Windows helper decoded this UTF-8 caveat as cp1252.
        # Repair only that known text import, never a value/oracle or saved file.
        for old_detector, new_detector in zip(expected['detectors'], actual['detectors']):
            for i, (caption, current_caption) in enumerate(zip(old_detector['caveats'], new_detector['caveats'])):
                if caption != current_caption:
                    assert current_caption.startswith('Signed Pearson correlation between ')
                    assert caption.encode('cp1252').decode('utf-8') == current_caption
                    old_detector['caveats'][i] = current_caption
                    legacy_caption_repairs += 1
        assert actual == expected, stem
        assert a == baseline['spectrogram'], stem
        stream, coverage = r['stream'], r['coverage']
        assert presentation['presentation_version'] == 1
        assert presentation['decoded_pcm_sha256'] == coverage['decoded_pcm_sha256']
        assert presentation['bitrate_method'] == 'encoded_complete_packet_mean'
        assert presentation['bitrate_bps'] == stream['sample_rate'] * stream['channels'] * 16
        assert 0 < presentation['bitrate_interval']['end_frame'] <= coverage['analyzed_frames']
        width, height, pixels, d = decode_png(path)
        assert (width, height) == sizes[size]
        assert d['method_id'] == 'alfred-calibrated-png-v1' and d['render_version'] == 1
        assert d['dpi'] == 300 and d['decoded_pcm_sha256'] == coverage['decoded_pcm_sha256']
        assert d['source_filename'] == source.name and d['title'] == source.name
        assert 'kbps (audio average)' in d['stream_line']
        assert 'Hann 1024 / hop 512' in d['analysis_line']
        assert ('FULL STREAM' if coverage['reached_end'] else 'ANALYZED PREFIX') in d['analysis_line']
        assert d['frequency_ticks'][0]['value'] == 0
        assert d['frequency_ticks'][-1]['value'] == stream['sample_rate'] / 2
        assert d['time_ticks'][0]['label'] == '0:00'
        assert d['time_ticks'][-1]['value'] == coverage['end_seconds']
        assert len({v['label'] for v in d['time_ticks']}) == len(d['time_ticks'])
        assert (d['legend_min_db'], d['legend_max_db']) == (-140, 0)
        for cutoff in d['cutoffs']:
            assert 'native Ch' in cutoff['label']
            for channel in cutoff['channel_indices']:
                assert cutoff['frequency_hz'] == r['channels'][channel]['spectral']['cutoff_p95_hz']
        if name == 'silent':
            assert not d['cutoffs']
        if name == 'anti':
            assert any('native channels contain signal' in s for s in d['warnings'])
        scale = width / 1280
        x, top, bottom = round(1128 * scale), round(150 * scale), round(590 * scale)
        assert pixels[(top * width + x) * 3:(top * width + x) * 3 + 3] == bytes([255, 240, 160])
        assert pixels[(bottom * width + x) * 3:(bottom * width + x) * 3 + 3] == bytes([0, 0, 0])
        (output / (stem + '.json')).write_text(process.stdout, encoding='utf-8')
        (output / (stem + '-canvas.json')).write_text(json.dumps(d, indent=2) + '\n', encoding='utf-8')
        receipts.append({'case': stem, 'dimensions': [width, height], 'rgb_sha256': hashlib.sha256(pixels).hexdigest(), 'png_sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'pcm_sha256': coverage['decoded_pcm_sha256'], 'bitrate_bps': presentation['bitrate_bps']})
    # Collision never replaces even a complete, previously exported canvas.
    path = output / 'tone8k.png'
    before = path.read_bytes()
    process = subprocess.run([str(binary), str(controls / 'tone8k.wav'), 'full', str(path)], capture_output=True, text=True, encoding='utf-8', timeout=60)
    assert process.returncode == 0
    assert json.loads(process.stderr)['path'] is None and path.read_bytes() == before
    # Strictly short artifact failure preserves a successful measurement.
    path = output / 'short.png'
    process = subprocess.run([str(binary), str(controls / 'short.wav'), 'full', str(path)], capture_output=True, text=True, encoding='utf-8', timeout=60)
    p, export = json.loads(process.stdout), json.loads(process.stderr)
    assert process.returncode == 0 and p['measurement']['status'] == 'analyzed'
    assert p['spectrogram']['status'] == 'unavailable' and export['path'] is None and not path.exists()
    summary = {'generated_pngs': receipts, 'native_and_artifact_v1_equal_frozen_p06': True, 'known_legacy_caveat_utf8_import_repairs': legacy_caption_repairs, 'collision_preserved': True, 'unavailable_path_null': True, 'independent_crc_filter_decode': True}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
    print(f'PASS: {len(receipts)} calibrated Rust PNGs, all 3 resolutions, independent RGB8/CRC/DPI/context decode, exact frozen native/artifact data and WAV bitrate, collision and short null path')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--controls', required=True, type=Path, help='Frozen generated outputs from check_spectrogram.py')
    parser.add_argument('--output', required=True, type=Path, help='New receipt/image directory')
    args = parser.parse_args()
    check(args.binary.resolve(), args.controls.resolve(), args.output.resolve())
