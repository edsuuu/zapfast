#!/usr/bin/env python3
"""Synthesize ZapFast's "Pop" notification sounds into assets/sounds/.

`pop.wav` is a short two-note bubble pop for new messages, in the style of
WhatsApp's own tone; `pop-mention.wav` adds a third, higher note for mentions
and replies. Both are drawn from sine waves here, sample by sample, so they
belong to this repository (MIT) and copy nothing. Standard library only.

    python3 scripts/generate-pop-sounds.py
"""
import math
import struct
import wave
from pathlib import Path

RATE = 48_000
ROOT = Path(__file__).resolve().parents[1]
SOUNDS = ROOT / 'assets/sounds'


def note(start, start_hz, end_hz, sweep, tau, level, harmonic=0.18):
    """A pop: a sine gliding from `start_hz` to `end_hz` over `sweep`
    seconds, a 3 ms attack and an exponential decay of time constant `tau`."""
    return (start, start_hz, end_hz, sweep, tau, level, harmonic)


def render(notes, length):
    samples = [0.0] * int(RATE * length)
    for start, start_hz, end_hz, sweep, tau, level, harmonic in notes:
        phase = 0.0
        first = int(start * RATE)
        for index in range(first, len(samples)):
            t = (index - first) / RATE
            glide = min(t / sweep, 1.0)
            # Ease out, so the pitch settles like a bubble that has burst.
            hz = start_hz + (end_hz - start_hz) * (1 - (1 - glide) ** 3)
            phase += 2 * math.pi * hz / RATE
            attack = min(t / 0.003, 1.0)
            envelope = attack * math.exp(-t / tau)
            if envelope < 1e-4 and t > 0.01:
                break
            tone = math.sin(phase) + harmonic * math.sin(2 * phase)
            samples[index] += level * envelope * tone
    # A short fade at the very end, then -3 dBFS at the peak.
    fade = int(0.01 * RATE)
    for index in range(fade):
        samples[-1 - index] *= index / fade
    peak = max(abs(sample) for sample in samples) or 1.0
    gain = 10 ** (-3 / 20) / peak
    return [sample * gain for sample in samples]


def write(name, samples):
    path = SOUNDS / name
    with wave.open(str(path), 'wb') as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(b''.join(
            struct.pack('<h', round(max(-1.0, min(1.0, sample)) * 32767))
            for sample in samples
        ))
    print(f'wrote {path.relative_to(ROOT)} ({len(samples) / RATE:.2f} s)')


MESSAGE = [
    note(0.000, 520, 880, 0.030, 0.045, 0.9),
    note(0.085, 1040, 1250, 0.020, 0.075, 0.75),
]
MENTION = MESSAGE + [
    note(0.170, 1400, 1580, 0.020, 0.090, 0.6),
]

if __name__ == '__main__':
    write('pop.wav', render(MESSAGE, 0.42))
    write('pop-mention.wav', render(MENTION, 0.55))
