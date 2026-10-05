"""Prepare known-chain synthetic controls, then evaluate frozen non-locked splits.

Requires local NumPy/FFmpeg for preparation. No recordings or network are used.
Never infer expected detector outcomes from the transformation labels.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time
import wave

ROOT = Path(__file__).resolve().parents[1]
DURATION = 4
# Separate procedural families, not independent real-world recordings. All edits
# and encodings of a family remain in the same group, including both codec labels.
GROUPS = [
    ('broadband', 'development', 44100, 6101),
    ('harmonic', 'development', 48000, 6102),
    ('attacks', 'validation', 44100, 6201),
    ('chirps', 'validation', 48000, 6202),
    ('multiband', 'locked_test', 44100, 6301),
    ('plucked', 'locked_test', 48000, 6302),
]
VARIANTS = ['native_s24', 'lowpass_s24', 'aac256_s24', 'aac256_trim137_s24',
            'aac256_s16', 'vorbisq5_s24', 'vorbisq5_s16', 'mp3192_s24']


def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def save(path, obj):
    with Path(path).open('x', encoding='utf-8') as handle:
        json.dump(obj, handle, indent=2, allow_nan=False)
        handle.write('\n')


def execute(command, log, binary=False, timeout=180):
    process = subprocess.run(list(map(str, command)), capture_output=True, timeout=timeout)
    Path(str(log) + '.stderr').write_bytes(process.stderr)
    if not binary:
        Path(str(log) + '.stdout').write_bytes(process.stdout)
    save(Path(str(log) + '.command.json'), {'command': list(map(str, command)), 'exit': process.returncode})
    if process.returncode:
        raise RuntimeError(f'Command failed ({process.returncode}); see {log}.stderr')
    return process.stdout


def generated_words(family, rate, seed):
    import numpy as np
    rng = np.random.default_rng(seed)
    n = rate * DURATION
    t = np.arange(n) / rate
    output = np.zeros((n, 2))
    for ch in range(2):
        noise = rng.standard_normal(n)
        if family == 'broadband':
            freq = np.fft.rfftfreq(n, 1 / rate)
            x = np.fft.irfft(np.fft.rfft(noise) / np.sqrt(np.maximum(freq, 60)), n=n)
            x *= 0.3 + 0.7 * np.sin(2 * np.pi * (0.7 + ch * 0.13) * t)**2
        elif family == 'harmonic':
            x = np.zeros(n)
            for harmonic in range(1, 40):
                x += np.sin(2 * np.pi * (183 + 19 * ch) * harmonic * t + rng.uniform(0, 6.28)) / harmonic**1.2
            x *= 0.3 + 0.7 * np.sin(np.pi * t)**2
            x += noise * 0.008
        elif family == 'attacks':
            x = noise * 0.002
            for start in (0.2, 0.62, 1.1, 1.75, 2.2, 2.9, 3.5):
                a, length = int(start * rate), int(0.3 * rate)
                decay = np.exp(-np.arange(length) / (rate * (0.025 + 0.012 * ch)))
                x[a:a+length] += noise[a:a+length] * decay
        elif family == 'chirps':
            x = 0.4 * np.sin(2 * np.pi * (130 * t + (19000 - 130) * t**2 / (2 * DURATION)) + ch * 0.9)
            x += 0.2 * np.sin(2 * np.pi * (17000 * t - 16000 * t**2 / (2 * DURATION)))
            x *= 0.2 + 0.8 * np.sin(2 * np.pi * 2.3 * t)**2
            x += noise * 0.03
        elif family == 'multiband':
            freq = np.fft.rfftfreq(n, 1 / rate)
            shape = ((freq > 200) & (freq < 4200)) * 0.6 + ((freq > 9000) & (freq < 18500)) * 0.3
            x = np.fft.irfft(np.fft.rfft(noise) * shape, n=n)
            x *= 0.25 + 0.75 * np.sin(2 * np.pi * (1.2 + 0.1 * ch) * t)**2
        elif family == 'plucked':
            x = noise * 0.0005
            for start, fundamental in zip((0.1, 0.9, 1.8, 2.7), (220, 329, 277, 415)):
                age = t - start
                gate = (age >= 0) * np.exp(-np.maximum(age, 0) * 4)
                for harmonic in range(1, 30):
                    x += gate * np.sin(2 * np.pi * fundamental * (1 + 0.002 * ch) * harmonic * age) / harmonic**1.4
        else:
            raise ValueError(f'Unknown family: {family}')
        x -= np.mean(x)
        output[:, ch] = x
    # One common scale preserves native channel differences. Leave ample headroom.
    output *= 0.72 / np.max(np.abs(output))
    return np.rint(output * 2**23).astype('<i4')


def write_wav(path, words, rate):
    import numpy as np
    with wave.open(str(path), 'wb') as handle:
        handle.setparams((2, 3, rate, 0, 'NONE', 'not compressed'))
        handle.writeframes(words.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    return hashlib.sha256((words << 8).astype('<i4').tobytes()).hexdigest()


def label(variant, codec):
    if variant not in VARIANTS or codec not in ('aac', 'vorbis'):
        raise ValueError('Unknown recipe variant or target codec')
    return 'present' if variant.startswith('aac' if codec == 'aac' else 'vorbis') else 'absent'


def verify(bundle):
    inventory = json.loads((bundle / 'inventory.json').read_text())
    for name, expected in inventory['sha256'].items():
        path = (bundle / name).resolve()
        if not path.is_relative_to(bundle.resolve()) or sha(path) != expected:
            raise ValueError(f'Frozen artifact changed: {name}')
    return inventory


def prepare(args):
    import numpy as np
    bundle = args.output.resolve()
    if bundle.exists() or not bundle.is_relative_to((ROOT / 'corpus/local').resolve()):
        raise ValueError('Output must be a new directory below corpus/local')
    bundle.mkdir(parents=True)
    for folder in ['bin', 'sources', 'audio', 'encoded', 'recipes', 'logs', 'analysis']:
        (bundle / folder).mkdir()
    # Snapshot source and executables before any detector report is generated.
    shutil.copy2(__file__, bundle / 'generator.py')
    shutil.copy2(args.binary, bundle / 'bin/audio-forensic.exe')
    shutil.copy2(args.runner, bundle / 'bin/evaluate_reports.exe')
    ffmpeg = Path(shutil.which('ffmpeg')).resolve()
    ffprobe = Path(shutil.which('ffprobe')).resolve()
    tools = {'python': sys.version, 'numpy': np.__version__, 'generator_sha256': sha(bundle / 'generator.py'),
             'ffmpeg_sha256': sha(ffmpeg), 'ffprobe_sha256': sha(ffprobe),
             'ffmpeg_version': execute([ffmpeg, '-version'], bundle / 'logs/ffmpeg-version').decode().splitlines()[0]}
    version = execute([bundle / 'bin/audio-forensic.exe', '--version'], bundle / 'logs/engine-version').decode().strip().split()[-1]
    design = {'design_version': 1, 'seconds': DURATION, 'groups': GROUPS, 'variants': VARIANTS,
              'tools': tools, 'engine_version': version,
              'scope': 'Known procedural chains; distinct signal families, not independent real recordings or population accuracy.',
              'locked_policy': 'Generate, fingerprint and independently decode only; no Rust analysis of locked_test.'}
    save(bundle / 'design.json', design)
    cases, audio_records = [], []
    for family, split, rate, seed in GROUPS:
        words = generated_words(family, rate, seed)
        source = bundle / f'sources/{family}.wav'
        direct_hash = write_wav(source, words, rate)
        recipe = {'group': family, 'split': split, 'rate': rate, 'seed': seed,
                  'source': str(source.relative_to(bundle)), 'direct_generated_pcm_sha256': direct_hash,
                  'generator_sha256': tools['generator_sha256'], 'design_sha256': sha(bundle / 'design.json'),
                  'operations': [], 'artifacts': [], 'history': 'Procedural PCM; no imported audio or prior codec stage.'}

        def ff(name, options):
            command = [ffmpeg, '-nostdin', '-v', 'error', '-xerror', '-n', *options]
            execute(command, bundle / f'logs/{family}_{name}')
            recipe['operations'].append({'name': name, 'command': list(map(str, command))})

        def export(name, parent, bits=24, filters=None):
            path = bundle / f'audio/{family}_{name}.wav'
            options = ['-i', parent, '-map', '0:a:0', '-map_metadata', '-1']
            if filters:
                options += ['-af', filters]
            ff(name, [*options, '-c:a', f'pcm_s{bits}le', path])
            return path

        export('native_s24', source)
        export('lowpass_s24', source, filters='lowpass=f=14000')
        aac = bundle / f'encoded/{family}.m4a'
        vorbis = bundle / f'encoded/{family}.ogg'
        mp3 = bundle / f'encoded/{family}.mp3'
        ff('encode_aac', ['-i', source, '-map_metadata', '-1', '-c:a', 'aac', '-aac_tns', '1', '-b:a', '256k', aac])
        ff('encode_vorbis', ['-i', source, '-map_metadata', '-1', '-c:a', 'libvorbis', '-q:a', '5', vorbis])
        ff('encode_mp3', ['-i', source, '-map_metadata', '-1', '-c:a', 'libmp3lame', '-b:a', '192k', mp3])
        aac24 = export('aac256_s24', aac)
        export('aac256_trim137_s24', aac24, filters='atrim=start_sample=137,asetpts=PTS-STARTPTS')
        export('aac256_s16', aac, bits=16)
        export('vorbisq5_s24', vorbis)
        export('vorbisq5_s16', vorbis, bits=16)
        export('mp3192_s24', mp3)
        aac_pcm = None
        for variant in VARIANTS:
            path = bundle / f'audio/{family}_{variant}.wav'
            probe_cmd = [ffprobe, '-v', 'error', '-select_streams', 'a:0', '-show_streams', '-of', 'json', path]
            metadata = json.loads(execute(probe_cmd, bundle / f'logs/{path.stem}_probe'))['streams'][0]
            bits = 16 if variant.endswith('s16') else 24
            if (int(metadata['sample_rate']), metadata['channels'], metadata['bits_per_sample']) != (rate, 2, bits):
                raise ValueError(f'Unexpected decoded geometry: {path}')
            pcm_cmd = [ffmpeg, '-nostdin', '-v', 'error', '-xerror', '-i', path, '-map', '0:a:0', '-c:a', 'pcm_s32le', '-f', 's32le', 'pipe:1']
            pcm = execute(pcm_cmd, bundle / f'logs/{path.stem}_independent-pcm', binary=True)
            pcm_hash = hashlib.sha256(pcm).hexdigest()
            if len(pcm) % 8:
                raise ValueError('Nonintegral stereo frame count')
            if variant == 'native_s24' and pcm_hash != direct_hash:
                raise ValueError('Independent decode differs from generated PCM')
            if variant == 'aac256_s24':
                aac_pcm = pcm
            if variant == 'aac256_trim137_s24' and pcm != aac_pcm[137 * 8:]:
                raise ValueError('Trim is not an exact 137-frame removal')
            record = {'id': path.stem, 'group': family, 'split': split, 'variant': variant,
                      'path': str(path.relative_to(bundle)), 'sha256': sha(path), 'pcm_sha256': pcm_hash,
                      'sample_rate': rate, 'channels': 2, 'bits': bits, 'frames': len(pcm) // 8,
                      'independent_command': list(map(str, pcm_cmd)),
                      'lineage': ['procedural_pcm'] + ([variant.split('_')[0]] if label(variant, 'aac') == 'present' or label(variant, 'vorbis') == 'present' or variant.startswith('mp3') else [])}
            recipe['artifacts'].append(record)
            audio_records.append(record)
        recipe_path = bundle / f'recipes/{family}.json'
        save(recipe_path, recipe)
        for record in recipe['artifacts']:
            for codec in ['aac', 'vorbis']:
                cases.append({'id': f'{record["id"]}_{codec}', 'source_group': family, 'split': split,
                              'codec': codec, 'label': label(record['variant'], codec), 'provenance': 'generated_control',
                              'processing': record['variant'], 'evidence_ref': str(recipe_path.relative_to(bundle)),
                              'evidence_sha256': sha(recipe_path), 'expected_pcm_sha256': record['pcm_sha256']})
        print(f'Prepared {family}: {len(VARIANTS)} independently decoded controls ({split})', flush=True)
    save(bundle / 'audio-index.json', audio_records)
    save(bundle / 'manifest.json', {'manifest_version': 1, 'report_engine_version': version, 'cases': cases})
    execute([bundle / 'bin/evaluate_reports.exe', 'freeze', bundle / 'manifest.json', bundle / 'plan.json'], bundle / 'logs/freeze')
    # Include original samples, codec intermediates, evidence, tools and plans.
    inventory = {str(p.relative_to(bundle)): sha(p) for p in sorted(bundle.rglob('*')) if p.is_file()}
    save(bundle / 'inventory.json', {'sha256': inventory, 'audio_files': len(audio_records), 'cases': len(cases),
                                   'locked_audio_files': sum(r['split'] == 'locked_test' for r in audio_records),
                                   'prepared': True, 'evaluated': False})
    verify(bundle)
    print(f'Frozen {len(audio_records)} files / {len(cases)} codec cases; no detector analysis yet.', flush=True)


def evaluate(args):
    bundle = args.bundle.resolve()
    if not bundle.is_relative_to((ROOT / 'corpus/local').resolve()):
        raise ValueError('Bundle must be beneath corpus/local')
    verify(bundle)
    run = bundle / 'analysis' / args.split
    run.mkdir()  # Never overwrite/reuse a partial evaluation.
    (run / 'reports').mkdir()
    (run / 'raw').mkdir()
    items = [r for r in json.loads((bundle / 'audio-index.json').read_text()) if r['split'] == args.split]
    records = []
    for item in items:
        started = time.monotonic()
        raw = execute([bundle / 'bin/audio-forensic.exe', '--json', bundle / item['path']], run / f'raw/{item["id"]}', timeout=600)
        array = json.loads(raw)
        if len(array) != 1:
            raise ValueError('Expected one report')
        report = array[0]
        coverage = report.get('coverage') or {}
        if (report['status'] != 'analyzed' or coverage.get('decoded_pcm_sha256') != item['pcm_sha256']
                or coverage.get('analyzed_frames') != item['frames'] or not coverage.get('reached_end')):
            raise ValueError(f'Analysis/integrity failure: {item["id"]}')
        for codec in ['aac', 'vorbis']:
            save(run / f'reports/{item["id"]}_{codec}.json', report)
        records.append({'id': item['id'], 'status': report['status'], 'exact_pcm': True,
                        'seconds': round(time.monotonic() - started, 4), 'report_sha256': sha(run / f'raw/{item["id"]}.stdout')})
        print(f'{args.split} {len(records)}/{len(items)}: {item["id"]}, exact PCM', flush=True)
    paths = sorted((run / 'reports').glob('*.json'))
    execute([args.schema_python, ROOT / 'scripts/validate_report_schema.py', *paths,
             '--output', run / 'schema-receipt.json'], run / 'schema')
    execute([bundle / 'bin/evaluate_reports.exe', 'run', bundle / 'plan.json', args.split,
             run / 'reports', run / 'evaluation.json'], run / 'evaluate')
    verify(bundle)
    # Reserved cases must not acquire reports as a side effect.
    if (bundle / 'analysis/locked_test').exists():
        raise ValueError('Unexpected locked evaluation directory')
    save(run / 'receipt.json', {'passed': True, 'split': args.split, 'files': records,
                               'plan_sha256': sha(bundle / 'plan.json'), 'inventory_sha256': sha(bundle / 'inventory.json'),
                               'reserved_locked_not_analyzed': True,
                               'report_sha256': {p.name: sha(p) for p in paths}})
    print(f'Completed {args.split}: {len(records)} files, {len(paths)} labeled cases; frozen artifacts preserved.', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    prep = sub.add_parser('prepare')
    prep.add_argument('--binary', type=Path, required=True)
    prep.add_argument('--runner', type=Path, required=True)
    prep.add_argument('--output', type=Path, required=True)
    run = sub.add_parser('evaluate')
    run.add_argument('--bundle', type=Path, required=True)
    run.add_argument('--split', choices=['development', 'validation'], required=True)
    run.add_argument('--schema-python', type=Path, required=True)
    args = parser.parse_args()
    (prepare if args.action == 'prepare' else evaluate)(args)


if __name__ == '__main__':
    main()
