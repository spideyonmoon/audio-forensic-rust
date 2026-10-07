"""Check frozen ALAC fixtures, CLI/schema serialization and metadata limits.

Development helper; no private audio, encoder install or runtime dependency.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess

import jsonschema

ROOT = Path(__file__).resolve().parents[1]


def box(kind, payload):
    return struct.pack('>I', len(payload) + 8) + kind + payload


def children(payload):
    at = 0
    while at < len(payload):
        size, kind = struct.unpack_from('>I4s', payload, at)
        assert size >= 8 and at + size <= len(payload)
        yield kind, payload[at + 8:at + size]
        at += size


def tagged(original, tags):
    # moov follows mdat, so replacing udta cannot change PCM chunk offsets.
    output = bytearray()
    for kind, payload in children(original):
        if kind == b'moov':
            payload = b''.join(box(k, p) for k, p in children(payload) if k != b'udta')
            payload += box(b'udta', box(b'meta', bytes(4) + box(b'ilst', b''.join(tags))))
        output.extend(box(kind, payload))
    return output


def tag(kind, value, dtype=1):
    return box(kind, box(b'data', struct.pack('>II', dtype, 0) + value))


def variant(blob, change):
    result = bytearray()
    for kind, payload in children(blob):
        if kind in [b'moov', b'trak', b'mdia', b'minf', b'stbl']:
            payload = variant(payload, change)
        if change == 'co64' and kind == b'stco':
            n = struct.unpack_from('>I', payload, 4)[0]
            payload = payload[:8] + b''.join(struct.pack('>Q', struct.unpack_from('>I', payload, 8 + i * 4)[0]) for i in range(n))
            kind = b'co64'
        elif change == 'sample-entry-v1' and kind == b'stsd':
            size, entry_kind = struct.unpack_from('>I4s', payload, 8)
            entry = bytearray(payload[16:])
            entry[8:10] = b'\x00\x01'
            entry[28:28] = bytes(16)
            payload = payload[:8] + box(entry_kind, entry)
        elif change == 'sync-and-zero-offset' and kind == b'stbl':
            payload += box(b'stss', bytes(4) + struct.pack('>III', 2, 1, 2))
            payload += box(b'ctts', bytes(4) + struct.pack('>III', 1, 2, 0))
        elif change == 'track-metadata' and kind == b'moov':
            parts = list(children(payload))
            udta = next(box(k, p) for k, p in parts if k == b'udta')
            payload = b''.join(box(k, p + udta if k == b'trak' else p) for k, p in parts if k != b'udta')
        if change == 'large-moov-header' and kind == b'moov':
            result.extend(struct.pack('>I4sQ', 1, kind, len(payload) + 16) + payload)
        else:
            result.extend(box(kind, payload))
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--metadata-reader', type=Path, required=True)
    parser.add_argument('--reference-reader', type=Path)
    parser.add_argument('--output', type=Path, default=ROOT / 'target/f01-alac/serialized')
    parser.add_argument('--large', action='store_true', help='Add a generated 700 MiB padding/seek control')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    fixtures = ROOT / 'tests/fixtures/alac'
    manifest = json.loads((fixtures / 'manifest.json').read_text())
    measurement_schema = json.loads((ROOT / 'schemas/analysis-report-0.18.0.schema.json').read_text())
    metadata_schema = json.loads((ROOT / 'schemas/metadata-1.schema.json').read_text())
    measurement = jsonschema.Draft202012Validator(measurement_schema)
    metadata = jsonschema.Draft202012Validator(metadata_schema)
    for case in manifest['cases']:
        assert hashlib.sha256((fixtures / case['file']).read_bytes()).hexdigest() == case['file_sha256']
    def analyze(paths, seconds=None):
        command = [str(args.binary.resolve()), '--json']
        if seconds is not None:
            command += ['--max-seconds', str(seconds)]
        command += [str(p) for p in paths]
        result = subprocess.run(command, capture_output=True, text=True)
        assert result.returncode in [0, 1], result.stderr
        reports = json.loads(result.stdout)
        for report in reports:
            measurement.validate(report)
            assert report['ancestry_verdict'] == 'INCONCLUSIVE' and report['evidence_index'] is None
        return reports
    reports = analyze([fixtures / case['file'] for case in manifest['cases']])
    for report, case in zip(reports, manifest['cases'], strict=True):
        assert report['status'] == 'analyzed', report['diagnostics']
        assert report['coverage']['decoded_pcm_sha256'] == case['pcm_sha256']
        assert report['coverage']['analyzed_frames'] == case['frames']
        assert report['coverage']['decoder_verification'] is None  # ALAC has no embedded PCM checksum.
    (args.output / 'full.json').write_text(json.dumps(reports, indent=2) + '\n')
    prefixes = []
    for case in manifest['cases']:
        if case['bits'] == 24 and case['channels'] == 2:
            report = analyze([fixtures / case['file']], 1000.25 / case['rate'])[0]
            assert report['status'] == 'analyzed' and not report['coverage']['reached_end']
            assert report['coverage']['decoded_pcm_sha256'] == case['prefix_pcm_sha256']
            prefixes.append(report)
    (args.output / 'prefix.json').write_text(json.dumps(prefixes, indent=2) + '\n')
    original = (fixtures / '8000-16-1-tail.m4a').read_bytes()
    variants = []
    for name in ['co64', 'sample-entry-v1', 'large-moov-header', 'sync-and-zero-offset', 'track-metadata']:
        path = args.output / f'{name}.m4a'
        path.write_bytes(variant(original, name))
        report = analyze([path])[0]
        assert report['status'] == 'analyzed', report['diagnostics']
        assert report['coverage']['decoded_pcm_sha256'] == manifest['cases'][0]['pcm_sha256']
        variants.append(report)
        meta = json.loads(subprocess.check_output([str(args.metadata_reader.resolve()), str(path)], text=True, encoding='utf-8'))
        metadata.validate(meta)
        assert meta['status'] == 'available'
        assert meta['entries'][meta['named_tags']['title'][0]]['value'] == 'Generated F01'
    (args.output / 'variants.json').write_text(json.dumps(variants, indent=2) + '\n')
    freeform = box(b'----', box(b'mean', bytes(4) + b'com.apple.iTunes') + box(b'name', bytes(4) + b'REPLAYGAIN_TRACK_GAIN') + box(b'data', struct.pack('>II', 1, 0) + b'-6.25 dB'))
    controls = {
        'tags': [tag(b'\xa9nam', 'Unicode বাংলা'.encode()), tag(b'\xa9nam', b'Duplicate'), freeform,
                 tag(b'tmpo', b'\x00\x78', 22), tag(b'zzzz', b'Unknown preserved'), tag(b'covr', b'\xff\xd8\xff\xd9', 13)],
        'invalid-utf8': [tag(b'\xa9nam', b'bad\xfftitle')],
        'utf16-opaque': [tag(b'\xa9nam', 'UTF16'.encode('utf-16be'), 2)],
        'long-value': [tag(b'\xa9cmt', b'x' * 20000)],
        'entry-cap': [tag(b'zzzz', b'x') for _ in range(1025)],
        'text-cap': [tag(b'zzzz', b'x' * 16384) for _ in range(70)],
    }
    meta_reports = {}
    for name, tags in controls.items():
        path = args.output / f'{name}.m4a'
        path.write_bytes(tagged(original, tags))
        result = subprocess.run([str(args.metadata_reader.resolve()), str(path)], check=True, capture_output=True, text=True, encoding='utf-8')
        report = json.loads(result.stdout)
        metadata.validate(report)
        assert report['status'] == 'available', report['diagnostics']
        limits = report['text_limits']
        assert limits['retained_text_utf8_bytes'] + limits['omitted_text_utf8_bytes'] == limits['total_text_utf8_bytes']
        assert limits['retained_text_utf8_bytes'] <= 1048576 and len(report['entries']) <= 1024
        assert all(len(e['key'].encode()) <= 256 and len((e['value'] or '').encode()) <= 16384 for e in report['entries'])
        if name == 'tags':
            assert len(report['named_tags']['title']) == 2
            assert report['entries'][report['named_tags']['replaygain_track_gain'][0]]['value'] == '-6.25 dB'
            assert report['entries'][report['named_tags']['bpm'][0]]['value'] == '120'
            assert report['entries'][report['named_tags']['bpm'][0]]['original_value_bytes'] == 2
            assert report['entries'][report['named_tags']['title'][0]]['original_key_bytes'] == 4
            assert report['artwork'][0]['byte_length'] == 4 and report['entries'][-1]['value'] is None
        elif name == 'invalid-utf8':
            assert limits['invalid_utf8_entries'] == 1 and not limits['complete']
        elif name == 'utf16-opaque':
            assert len(report['opaque_metadata']) == 1 and not report['observations']['text_scan_complete']
        elif name == 'long-value':
            assert limits['truncated_entries'] == 1 and report['entries'][0]['value_truncated']
        elif name == 'entry-cap':
            assert limits['total_entries'] == 1025 and limits['omitted_entries'] == 1
        elif name == 'text-cap':
            assert limits['retained_text_utf8_bytes'] == 1048576 and limits['omitted_entries'] > 0
        meta_reports[name] = report
        # The same metadata preflight also admits audio decoding without altering PCM.
        audio = analyze([path])[0]
        assert audio['status'] == 'analyzed', audio['diagnostics']
        assert audio['coverage']['decoded_pcm_sha256'] == manifest['cases'][0]['pcm_sha256']
    (args.output / 'metadata.json').write_text(json.dumps(meta_reports, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    # AAC is generated from known PCM: format identity never follows the suffix.
    aac = args.output / 'actual-aac.m4a'
    subprocess.run(['ffmpeg', '-v', 'error', '-nostdin', '-y', '-i', str(fixtures / manifest['cases'][0]['file']), '-c:a', 'aac', str(aac)], check=True)
    report = analyze([aac])[0]
    assert report['status'] == 'unsupported' and report['coverage'] is None
    assert any('not ALAC' in d['message'] for d in report['diagnostics'])
    folder = args.output / 'directory-control'
    folder.mkdir(exist_ok=True)
    (folder / 'a-actual-aac.M4A').write_bytes(aac.read_bytes())
    (folder / 'b-native-alac.m4a').write_bytes(original)
    folder_reports = analyze([folder])
    assert [r['status'] for r in folder_reports] == ['unsupported', 'analyzed']
    assert folder_reports[0]['coverage'] is None
    assert folder_reports[1]['coverage']['decoded_pcm_sha256'] == manifest['cases'][0]['pcm_sha256']
    summary = {'full_schema_reports': len(reports), 'prefix_schema_reports': len(prefixes), 'metadata_controls': len(controls),
               'container_variants': len(variants), 'actual_aac': report, 'fixture_hashes_unchanged': True,
               'directory_reports': folder_reports}
    if args.reference_reader:
        reference_schema = json.loads((ROOT / 'schemas/reference-inputs-2.schema.json').read_text())
        envelope = json.loads(subprocess.check_output([str(args.reference_reader.resolve()), str(fixtures / '8000-24-2-front.m4a')], text=True))
        jsonschema.Draft202012Validator(reference_schema).validate(envelope)
        assert envelope['measurement']['status'] == 'analyzed'
        case = next(c for c in manifest['cases'] if c['file'] == '8000-24-2-front.m4a')
        assert envelope['measurement']['coverage']['decoded_pcm_sha256'] == case['pcm_sha256']
        assert envelope['reference_inputs'] is not None
        (args.output / 'reference-inputs.json').write_text(json.dumps(envelope, indent=2) + '\n')
        summary['three_pass_reference_inputs'] = True
    if args.large:
        path = args.output / 'generated-700MiB.m4a'
        target_size = 700 * 1024 * 1024
        padding = target_size - len(original)
        with path.open('wb') as out:
            for kind, payload in children(original):
                if kind == b'moov':
                    out.write(struct.pack('>I4s', padding, b'free'))
                    out.seek(padding - 8, 1)
                out.write(box(kind, payload))
        assert path.stat().st_size == target_size
        large = analyze([path])[0]
        assert large['status'] == 'analyzed', large['diagnostics']
        assert large['coverage']['decoded_pcm_sha256'] == manifest['cases'][0]['pcm_sha256']
        summary['large_padding_control_bytes'] = target_size
        summary['large_padding_report'] = large
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print('PASS:', len(reports), 'full +', len(prefixes), 'prefix reports; 6 bounded metadata controls and actual AAC rejection')


if __name__ == '__main__':
    main()
