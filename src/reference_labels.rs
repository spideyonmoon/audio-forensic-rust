#![allow(clippy::needless_return)] // Preserve the pinned audit function structure.
// Generated from pinned Python audit labels; see generate_reference_labels.py.
pub(super) fn variance(var: f64, legit_cutoff: bool) -> &'static str {
    if var < 1000.0 {
        return "[rigid/encoded-like]";
    }
    if var < 10000.0 {
        return "[stable: normal for mastered audio]";
    }
    if var < 100000.0 {
        return "[moderate: natural organic fluctuation]";
    }
    if var < 1000000.0 {
        return if legit_cutoff {
            "[high variation: organic/analog source]"
        } else {
            "[erratic cutoff: typical of VBR lossy encoders like AAC/Opus]"
        };
    }
    return if legit_cutoff {
        "[very high variation: complex analog source]"
    } else {
        "[erratic cutoff: typical of VBR lossy encoders like AAC/Opus]"
    };
}
pub(super) fn sharpness(s: f64) -> &'static str {
    if s < 2.0 {
        return "[gradual: natural EQ / mastering]";
    }
    if s < 5.0 {
        return "[moderate: normal variation]";
    }
    if s < 15.0 {
        return "[steep: algorithmic filter possible]";
    }
    return "[sharp cliff: hard mathematical low-pass filter]";
}
pub(super) fn hf_ratio(r: f64) -> &'static str {
    if r < 0.005 {
        return "[energy depletion: possible aggressive filter]";
    }
    if r < 0.015 {
        return "[low: typical for mastered/pop audio]";
    }
    if r < 0.05 {
        return "[moderate: normal mastered audio]";
    }
    return "[rich: full-spectrum, dynamic recording]";
}
pub(super) fn banding(b: f64) -> &'static str {
    if b < 0.7 {
        return "[minimal: no heavy quantization artifacts]";
    }
    if b < 0.85 {
        return "[moderate: normal for 16-bit PCM]";
    }
    if b < 0.95 {
        return "[strong: expected in PCM sources]";
    }
    return "[severe: heavy quantization detected]";
}
pub(super) fn nf(nf: f64) -> &'static str {
    if nf < -80.0 {
        return "[silent void: suspicious digital cutoff]";
    }
    if nf < -55.0 {
        return "[very quiet: typical digital silence]";
    }
    if nf < -35.0 {
        return "[moderate: natural dither or tape hiss]";
    }
    return "[loud: heavy analog noise or DSD shaping]";
}
pub(super) fn side(a: f64) -> &'static str {
    if a < 0.15 {
        return "[healthy: wide, complex stereo]";
    }
    if a < 0.3 {
        return "[normal: typical stereo imaging]";
    }
    if a < 0.5 {
        return "[mild depletion: acceptable joint stereo]";
    }
    if a < 0.7 {
        return "[moderate anomaly: heavy joint stereo]";
    }
    return "[severe anomaly: artificial stereo width or heavy compression]";
}
pub(super) fn entropy(e: f64, legit_cutoff: bool) -> &'static str {
    if e < 7.0 {
        return "[low: simple/tonal content]";
    }
    if e < 8.5 {
        return "[moderate: typical music complexity]";
    }
    if e < 9.5 {
        return if legit_cutoff {
            "[high: complex/dynamic content]"
        } else {
            "[high entropy: lossy noise-shaping / VBR footprint]"
        };
    }
    return if legit_cutoff {
        "[very high: noise-like complexity]"
    } else {
        "[very high entropy: lossy ultrasonic noise / dithering]"
    };
}
pub(super) fn bound(ny: f64, avg_bound: f64) -> &'static str {
    if avg_bound <= 0.0 {
        return "";
    }
    if avg_bound >= (ny * 0.85) {
        return "[organic scatter to the ceiling: lossless-like]";
    }
    if avg_bound >= 16500.0 {
        return "[moderate bound: high-bitrate encode or dark master]";
    }
    return "[scatter collapse: statistical void left by a lossy codec]";
}
pub(super) fn phase_entropy(e: f64, legit_cutoff: bool) -> &'static str {
    if e <= 0.0 {
        return "";
    }
    if e < 4.0 {
        return "[structured HF phase: tonal/organic]";
    }
    if e < 4.5 {
        return "[typical phase complexity]";
    }
    if legit_cutoff {
        return "[high but full-spectrum: dither/noise content]";
    }
    return "[quantized high-band phase: codec disruption]";
}
pub(super) fn sparsity(s: f64, legit_cutoff: bool) -> &'static str {
    if s < 0.05 {
        return "[dense spectrum: no psychoacoustic holes]";
    }
    if s < 0.3 {
        return "[some quiet bins: normal for dynamic audio]";
    }
    if legit_cutoff {
        return "[sparse but full-bandwidth: very dynamic content]";
    }
    return "[psychoacoustic holes below cutoff: codec bin-zeroing]";
}
pub(super) fn ultra_corr(c: f64) -> &'static str {
    if c > 0.6 {
        return "[HF breathes with the music: genuine harmonics]";
    }
    if c > 0.3 {
        return "[moderate coupling: normal]";
    }
    if c > 0.15 {
        return "[weak coupling: noisy or dark HF]";
    }
    return "[HF independent of music: dither, hiss, or injected fake noise]";
}
pub(super) fn mdct(score: f64) -> &'static str {
    if score < 0.0 {
        return "n/a (requires 44.1/48 kHz and sufficient active audio)";
    }
    if score < 0.06 {
        return "✓ no MDCT quantization lattice — clean coefficient statistics";
    }
    if score < 0.1 {
        return "~ faint coefficient clustering";
    }
    return "⚠ MDCT quantization lattice — AAC transcode signature";
}
