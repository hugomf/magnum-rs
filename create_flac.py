import numpy as np
import subprocess

# --- Audio Configuration ---
sr = 44100  # Sample rate
bpm = 130   # A classic, lively tempo
beat_dur = 60.0 / bpm

# --- Musical Notes ---
# FIX: Added the missing octave 2 notes for the bass guitar.
note_freqs = {
    'C2': 65.41, 'D2': 73.42, 'E2': 82.41, 'F2': 87.31, 'G2': 98.00, 'A2': 110.00, 'B2': 123.47,
    'C3': 130.81, 'D3': 146.83, 'E3': 164.81, 'F3': 174.61, 'G3': 196.00, 'A3': 220.00, 'B3': 246.94,
    'C4': 261.63, 'D4': 293.66, 'E4': 329.63, 'F4': 349.23, 'G4': 392.00, 'A4': 440.00, 'B4': 493.88, 'Bb4': 466.16,
    'C5': 523.25, 'D5': 587.33, 'E5': 659.25, 'F5': 698.46, 'G5': 783.99,
}

# --- THE ICONIC LA CUCARACHA MELODY ---
# This is the famous tune everyone knows.
melody_seq = [
    ('G4', 1.0), ('G4', 1.0), ('A4', 1.0), ('B4', 1.0), ('C5', 2.0), ('B4', 1.0), ('A4', 1.0), ('G4', 2.0),
    ('F4', 1.0), ('F4', 1.0), ('G4', 1.0), ('A4', 1.0), ('B4', 2.0), ('A4', 1.0), ('G4', 1.0), ('F4', 2.0),
    ('E4', 1.0), ('E4', 1.0), ('F4', 1.0), ('G4', 1.0), ('A4', 2.0), ('G4', 1.0), ('F4', 1.0), ('E4', 2.0),
    ('D4', 1.0), ('D4', 1.0), ('E4', 1.0), ('F4', 1.0), ('G4', 2.0), ('F4', 1.0), ('E4', 1.0), ('D4', 2.0),
    # Repeat for a longer song
    ('G4', 1.0), ('G4', 1.0), ('A4', 1.0), ('B4', 1.0), ('C5', 2.0), ('B4', 1.0), ('A4', 1.0), ('G4', 2.0),
    ('F4', 1.0), ('F4', 1.0), ('G4', 1.0), ('A4', 1.0), ('B4', 2.0), ('A4', 1.0), ('G4', 1.0), ('F4', 2.0),
]

# --- Setup Audio Channels ---
total_beats = sum(d for _, d in melody_seq)
total_dur = total_beats * beat_dur + 2.0  # Add 2 seconds of silence at the end
total_samples = int(total_dur * sr)
channels = [np.zeros(total_samples, dtype=np.float32) for _ in range(8)]

# --- Helper Function to Add Sounds ---
def add_wave(ch_idx, start_sec, freq, dur_sec, amp=0.7, shape='sine', harmonics=1, attack=0.02, decay=0.05):
    if freq <= 0 or dur_sec <= 0: return
    start = int(start_sec * sr)
    n = int(dur_sec * sr)
    if start + n > total_samples: n = total_samples - start
    if n <= 0: return

    t = np.linspace(0, dur_sec, n, False)
    
    # Generate waveform
    if shape == 'sine':
        wave = np.sin(2 * np.pi * freq * t)
    elif shape == 'square':
        wave = np.sign(np.sin(2 * np.pi * freq * t))
    elif shape == 'saw':
        wave = 2 * (freq * t % 1) - 1
    else:
        wave = np.sin(2 * np.pi * freq * t)
    
    # Add harmonics for richness
    for h in range(2, harmonics + 1):
        wave += (1.0 / h**2) * np.sin(2 * np.pi * freq * h * t)
    
    # Apply envelope (AD)
    attack_samples = int(attack * sr)
    decay_samples = int(decay * sr)
    env = np.ones(n)
    if attack_samples > 0:
        env[:attack_samples] = np.linspace(0, 1, attack_samples)
    if decay_samples > 0:
        env[-decay_samples:] = np.linspace(1, 0, decay_samples)
        
    channels[ch_idx][start:start + n] += amp * wave * env[:n]

# --- Build the 8-Channel Mariachi Band ---
current_time = 0.0
for note, beats in melody_seq:
    dur = beats * beat_dur
    f = note_freqs.get(note, 440)

    # CH 1 (Front Left): Lead Trumpet - The main melody
    add_wave(0, current_time, f, dur, amp=0.8, shape='saw', harmonics=5, attack=0.01, decay=0.1)

    # CH 2 (Front Right): Accordion - Harmonizes with the melody
    add_wave(1, current_time, f * 0.5, dur, amp=0.6, shape='saw', harmonics=3) # Lower octave
    # Add vibrato effect
    t = np.arange(int(current_time * sr), int((current_time + dur) * sr))
    if len(t) > 0:
        channels[1][t] *= (1 + 0.05 * np.sin(2 * np.pi * 5 * t / sr))

    # CH 3 (Center): Lead Vocals (simulated) - Follows melody
    add_wave(2, current_time, f, dur, amp=0.5, shape='sine', harmonics=2, attack=0.05)

    # CH 4 (LFE/Sub): Guitarrón (Bass) - Plays the root note of the chord
    bass_note_map = {'G4': 'G2', 'A4': 'A2', 'B4': 'B2', 'C5': 'C3', 'F4': 'F2', 'E4': 'E2', 'D4': 'D2'}
    bass_note = bass_note_map.get(note, 'G2')
    add_wave(3, current_time, note_freqs[bass_note], dur, amp=1.0, shape='square')

    # CH 5 (Rear Left): Rhythm Guitar - Arpeggios
    if note in ['G4', 'C5', 'E4']:
        add_wave(4, current_time, note_freqs['G3'], dur * 0.25, amp=0.3, shape='sine')
        add_wave(4, current_time + dur * 0.25, note_freqs['B3'], dur * 0.25, amp=0.3, shape='sine')
        add_wave(4, current_time + dur * 0.5, note_freqs['D4'], dur * 0.25, amp=0.3, shape='sine')
        add_wave(4, current_time + dur * 0.75, note_freqs['G4'], dur * 0.25, amp=0.3, shape='sine')

    # CH 6 (Rear Right): Violins - High-pitched harmony
    add_wave(5, current_time, f * 2, dur, amp=0.4, shape='sine', harmonics=4, attack=0.1)

    # CH 7 (Rear Center): Trumpet 2 - Fills and short stabs
    if beats == 2.0: # On longer notes, add a fill
        fill_freq = f * 1.5 # A fifth interval
        add_wave(6, current_time + dur * 0.5, fill_freq, dur * 0.3, amp=0.5, shape='saw', harmonics=4)

    # CH 8 (Side Right): Percussion Kit - All drums and shakers here
    beat_start = int(current_time / beat_dur)
    for i in range(int(beats)):
        beat_time = current_time + i * beat_dur
        # Kick on 1 and 3
        if i % 2 == 0:
            add_wave(7, beat_time, 60, 0.1, amp=1.2, shape='sine')
        # Snare on 2 and 4
        else:
            add_wave(7, beat_time, 200, 0.08, amp=0.7, shape='sine')
            # Add snare noise
            noise_len = int(0.05 * sr)
            noise = np.random.uniform(-1, 1, noise_len) * np.exp(-np.linspace(0, 5, noise_len))
            start_idx = int(beat_time * sr)
            if start_idx + noise_len < total_samples:
                channels[7][start_idx:start_idx + noise_len] += 0.5 * noise
        # Hi-hats on every beat
        hat_len = int(0.03 * sr)
        noise = np.random.uniform(-0.5, 0.5, hat_len) * np.exp(-np.linspace(0, 10, hat_len))
        start_idx = int(beat_time * sr)
        if start_idx + hat_len < total_samples:
            channels[7][start_idx:start_idx + hat_len] += 0.3 * noise

    current_time += dur

# --- Normalize and Encode ---
audio = np.stack(channels, axis=1)
max_val = np.max(np.abs(audio))
if max_val > 0:
    audio /= max_val # Normalize to prevent clipping
audio_int16 = (audio * 32767).astype(np.int16)

print("🎙️  Encoding 8-channel 7.1 FLAC with FFmpeg...")
process = subprocess.Popen([
    'ffmpeg', '-y',
    '-f', 's16le',
    '-ar', str(sr),
    '-ac', '8',
    '-channel_layout', '7.1',
    '-i', 'pipe:0',
    '-c:a', 'flac',
    '-compression_level', '8',
    'la_cucaracha_8ch.flac'
], stdin=subprocess.PIPE)

process.communicate(input=audio_int16.tobytes())

if process.returncode == 0:
    print("\n✅ ¡Listo! la_cucaracha_8ch.flac created successfully!")
    print("   🎺 Channel Layout:")
    print("      FL: Lead Trumpet (Main Melody)")
    print("      FR: Accordion")
    print("      C : Lead Vocals")
    print("      LFE: Guitarrón (Bass)")
    print("      RL: Rhythm Guitar")
    print("      RR: Violins")
    print("      RC: Trumpet 2 (Fills)")
    print("      SR: Percussion Kit (Drums & Shakers)")
    print("\n   Play with VLC → Audio → Audio Device → your 7.1 system")
else:
    print("❌ FFmpeg error. Make sure ffmpeg is in your PATH.")