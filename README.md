# Crimson

A warm metallic vocal chorus plugin, written from scratch in Rust.

Crimson is a stereo chorus voiced for modern vocals. A modulated delay line,
a breathing low-pass filter that tracks the LFO, a fixed high-pass to keep the
low end tight, and a light feedback path combine into a warm, metallic shimmer
that sits with pitch-corrected vocals rather than fighting them.

**Formats:** VST3 · CLAP
**Platforms:** macOS · Windows · Linux
**Status:** 1.0 beta

## What's interesting here

This is a real-time audio plugin with all of its DSP written by hand — no
wrapper around an existing engine.

- **Per-channel delay lines** with a fixed base delay plus LFO-modulated depth,
  keeping the effect in true chorus territory (never collapsing to a
  through-zero flanger).
- **Fractional-delay reads** with linear interpolation from a hand-rolled
  circular buffer.
- **Sample-rate-independent filters** — the low-pass and high-pass coefficients
  are derived from cutoff frequency and sample rate at runtime, so the plugin
  sounds identical at 44.1, 48, and 96 kHz.
- **A breathing low-pass** whose cutoff is swept geometrically by the LFO,
  deliberately brightening at longer delays (the opposite of a vintage
  bucket-brigade chorus) for a distinct timbral movement.
- **Band-limited LFO shapers** — sine, triangle, square, and sawtooth are each
  synthesized from a truncated Fourier series and peak-normalized, so switching
  shapes never changes the modulation depth or steps the delay line.
- **A stable feedback path** — the filter chain sits inside the loop, so every
  regeneration is darker (warm, not harsh) and low-frequency energy can't
  accumulate across repeats. Loop gain stays below unity by construction.
- **Real-time safe:** no allocation on the audio thread, denormal flushing to
  keep the CPU from stalling during silence, and clean state reset on transport
  stop.

## Controls

| Control   | Range        | Default  |
|-----------|--------------|----------|
| Rate      | 0.01–3 Hz    | 1.60 Hz  |
| Depth     | 0.5–6 ms     | 4.1 ms   |
| Feedback  | 0–90%        | 44%      |
| Mix       | 0–100%       | 40%      |
| Waveform  | Sine/Tri/Sq/Saw | Sine  |

## Building

Crimson is built with [nih-plug](https://github.com/robbert-vdh/nih-plug).

```
cargo xtask bundle chorus --release
```

Bundles are written to `target/bundled/` as `Crimson.clap` and `Crimson.vst3`.

## Install

Copy the format your DAW uses into the matching folder:

- **macOS:** `~/Library/Audio/Plug-Ins/VST3/` or `~/Library/Audio/Plug-Ins/CLAP/`
- **Windows:** `%COMMONPROGRAMFILES%\VST3\` or `%COMMONPROGRAMFILES%\CLAP\`
- **Linux:** `~/.vst3/` or `~/.clap/`

Logic Pro and GarageBand are not supported (they require the AU format).

## License

GPLv3. See [LICENSE](LICENSE).

Built on nih-plug, whose VST3 support requires GPLv3-compatible licensing.
