"""Generated-only F01 ALAC controls. Freeze original PCM before encoding.

Development-only FFmpeg oracle; ordinary Rust tests read these saved fixtures.
Never overwrites a frozen fixture set unless --replace is explicitly supplied.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import wave

RATES = [8000, 44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000]
FRAMES = 5001
PREFIX = 1000


def pcm(bits, channels, frames=FRAMES):
    packed = bytearray()
    canonical = bytearray()
    state = 0xF010CAFE
    for frame in range(frames):
        for channel in range(channels):
            state = (1664525 * state + 1013904223) & 0xFFFFFFFF
            word = (state >> (32 - bits)) - (1 << (bits - 1))
            if frame < 6:
                word = [-(1 << (bits - 1)), (1 << (bits - 1)) - 1, 0, 1, -1, 2][frame]
                if channel:
                    word = max(-(1 << (bits - 1)), min((1 << (bits - 1)) - 1, -word))
            packed.extend(word.to_bytes(bits // 8, 'little', signed=True))
            canonical.extend(struct.pack('<i', word << (32 - bits)))
    return packed, canonical


def seek_control(output):
    path = output / 'seek-long.m4a'
    if path.exists():
        raise SystemExit('Frozen seek fixture exists; choose a new output directory.')
    output.mkdir(parents=True, exist_ok=True)
    raw, expected = pcm(24, 2, 100000)
    with tempfile.TemporaryDirectory() as temp:
        wav = Path(temp) / 'seek.wav'
        with wave.open(str(wav), 'wb') as out:
            out.setnchannels(2)
            out.setsampwidth(3)
            out.setframerate(48000)
            out.writeframes(raw)
        command = ['ffmpeg', '-v', 'error', '-nostdin', '-y', '-i', str(wav), '-c:a', 'alac', str(path)]
        subprocess.run(command, check=True)
        actual = subprocess.check_output(['ffmpeg', '-v', 'error', '-xerror', '-i', str(path), '-f', 's32le', '-c:a', 'pcm_s32le', '-'])
        assert actual == expected
        receipt = {'file': path.name, 'frames': 100000, 'rate': 48000, 'bits': 24, 'channels': 2,
                   'pcm_sha256': hashlib.sha256(expected).hexdigest(), 'file_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                   'encoder_command': command, 'ffmpeg': subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0]}
        (output / 'seek.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print('Long seek control matches original and independent FFmpeg PCM exactly')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', type=Path, default=Path('tests/fixtures/alac'))
    parser.add_argument('--replace', action='store_true')
    parser.add_argument('--seek-only', action='store_true')
    args = parser.parse_args()
    if args.seek_only:
        seek_control(args.output)
        return
    if (args.output / 'manifest.json').exists() and not args.replace:
        raise SystemExit('Frozen fixtures exist; use a new output directory to compare reproduction.')
    args.output.mkdir(parents=True, exist_ok=True)
    receipt = {'ffmpeg': subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0],
               'domain': 'interleaved MSB-aligned signed s32le', 'frames': FRAMES, 'cases': []}
    with tempfile.TemporaryDirectory() as temp:
        temp = Path(temp)
        for rate in RATES:
            for bits in [16, 24]:
                for channels in [1, 2]:
                    raw, expected = pcm(bits, channels)
                    wav = temp / 'source.wav'
                    with wave.open(str(wav), 'wb') as out:
                        out.setnchannels(channels)
                        out.setsampwidth(bits // 8)
                        out.setframerate(rate)
                        out.writeframes(raw)
                    for layout in ['tail', 'front']:
                        name = f'{rate}-{bits}-{channels}-{layout}.m4a'
                        path = args.output / name
                        command = ['ffmpeg', '-v', 'error', '-nostdin', '-y', '-i', str(wav), '-map', '0:a:0', '-c:a', 'alac',
                                   '-metadata', 'title=Generated F01', '-metadata', 'comment=Known generated PCM',
                                   '-movflags', '+faststart' if layout == 'front' else '+use_metadata_tags', str(path)]
                        # Keep both layouts in the ordinary iTunes ilst metadata domain.
                        if layout == 'tail':
                            command[-3:-1] = []
                        subprocess.run(command, check=True)
                        actual = subprocess.check_output(['ffmpeg', '-v', 'error', '-xerror', '-nostdin', '-i', str(path),
                                                          '-map', '0:a:0', '-f', 's32le', '-c:a', 'pcm_s32le', '-'])
                        if actual != expected:
                            raise RuntimeError(f'Independent PCM differs for {name}')
                        probe = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_streams', '-of', 'json', str(path)]))
                        stream = probe['streams'][0]
                        assert stream['codec_name'] == 'alac' and int(stream['sample_rate']) == rate
                        assert int(stream['bits_per_raw_sample']) == bits and stream['channels'] == channels
                        receipt['cases'].append({'file': name, 'rate': rate, 'bits': bits, 'channels': channels,
                            'frames': FRAMES, 'prefix_frames': PREFIX, 'pcm_sha256': hashlib.sha256(expected).hexdigest(),
                            'prefix_pcm_sha256': hashlib.sha256(expected[:PREFIX * channels * 4]).hexdigest(),
                            'file_sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'encoder_command': command})
    (args.output / 'manifest.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print(f'{len(receipt["cases"])} ALAC controls matched original and independent FFmpeg PCM exactly')


if __name__ == '__main__':
    main()
