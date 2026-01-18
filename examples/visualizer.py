import math
import time
import wave
from typing import List, Tuple

import numpy as np
from pie.audio import AudioSource  # pyright: ignore[reportMissingModuleSource]
from pie.engine import Engine  # pyright: ignore[reportMissingModuleSource]
from pie.entity import Camera, Entity  # pyright: ignore[reportMissingModuleSource]
from pie.light import Light  # pyright: ignore[reportMissingModuleSource]
from pie.mesh import Mesh  # pyright: ignore[reportMissingModuleSource]
from pie.texture import Texture  # pyright: ignore[reportMissingModuleSource]
from scipy import interpolate as interp

WIDTH = 1280
HEIGHT = 720
WAV_PATH = "adenosine.wav"

# bars
NUM_BARS = 128
FFT_WINDOW = 4096
SCALE_FACTOR = 7.0
MIN_BAR_HEIGHT = 0.05
BARS_Y_OFFSET = -13.0

# timing
FPS = 120.0
DT = 1.0 / FPS

# light tuning
BASS_FREQUENCY_CUTOFF = 320.0

# osc
NUM_OSC_POINTS = 128
OSC_WINDOW = 2048
OSC_BASE_Y = 10.0
OSC_AMPLITUDE = 3.0
OSC_SCALE_BASE = 0.35
OSC_SMOOTH = 0.5
OSC_ICO_SUBDIV = 2


def read_wav_mono(path: str) -> Tuple[np.ndarray, int]:
    """Read WAV and return mono float32 samples in [-1, 1] and sample rate."""
    with wave.open(path, "rb") as wf:
        nchannels = wf.getnchannels()
        sampwidth = wf.getsampwidth()
        framerate = wf.getframerate()
        nframes = wf.getnframes()
        raw = wf.readframes(nframes)

    if sampwidth == 1:
        data = np.frombuffer(raw, dtype=np.uint8).astype(np.float32)
        data = (data - 128.0) / 128.0
    elif sampwidth == 2:
        data = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
    elif sampwidth == 4:
        data = np.frombuffer(raw, dtype=np.int32).astype(np.float32) / 2147483648.0
    else:
        raise ValueError(f"Unsupported sample width: {sampwidth}")

    if nchannels > 1:
        data = data.reshape(-1, nchannels).mean(axis=1)

    return data, framerate


def clamp(v: float, a: float, b: float) -> float:
    return max(a, min(b, v))


def main():
    engine = Engine("pie", WIDTH, HEIGHT)
    engine.configure_point_lights(0)

    camera = Camera(0.0, 0.0, 30.0)
    engine.camera = camera
    camera.fov = 60.0

    # directional light (global scene light)
    directional_light = Light.directional(1.0, 1.0, 1.0)
    engine.add_light(directional_light)
    directional_light.position = (0.0, -1.0, 0.0)

    bars: List[Entity] = []
    spacing = 0.35
    start_x = -((NUM_BARS - 1) * spacing) / 2.0
    for i in range(NUM_BARS):
        e = Entity()
        mesh = Mesh.cube()
        mesh.set_texture(Texture.from_color(0, 255, 0, 255))
        e.set_mesh(mesh)
        x = start_x + i * spacing
        e.scale = (0.35, MIN_BAR_HEIGHT, 0.35)
        e.position = (x, MIN_BAR_HEIGHT / 2.0 + BARS_Y_OFFSET, 0.0)
        engine.add_entity(e)
        bars.append(e)

    osc_spheres: List[Entity] = []
    osc_width = (NUM_BARS - 1) * spacing
    for i in range(NUM_OSC_POINTS):
        e = Entity()
        mesh = Mesh.icosphere(OSC_ICO_SUBDIV)
        mesh.set_texture(Texture.from_color(0, 118, 255, 255))
        e.set_mesh(mesh)
        x = start_x + (i / (NUM_OSC_POINTS - 1)) * osc_width
        e.scale = (OSC_SCALE_BASE, OSC_SCALE_BASE, OSC_SCALE_BASE)
        e.position = (x, OSC_BASE_Y, 0.0)
        engine.add_entity(e)
        osc_spheres.append(e)

    # load audio
    try:
        samples, sr = read_wav_mono(WAV_PATH)
    except Exception as ex:
        print(f"Error reading WAV '{WAV_PATH}': {ex}")
        return

    audio = AudioSource.from_wav(WAV_PATH, False)
    engine.add_audio_source(audio)
    audio.positional = False

    # fft setup
    fft_size = max(1024, int(FFT_WINDOW))
    win = np.hanning(fft_size)

    f_min = 20.0
    f_max = sr / 2.0
    log_freqs = np.logspace(math.log10(f_min), math.log10(f_max), NUM_BARS + 1)
    band_freqs = np.sqrt(log_freqs[:-1] * log_freqs[1:])

    freqs = np.fft.rfftfreq(fft_size, d=1.0 / sr)
    bass_bin_mask = freqs <= BASS_FREQUENCY_CUTOFF
    if not np.any(bass_bin_mask):
        bass_bin_mask[0] = True

    smoothed = np.zeros(NUM_BARS, dtype=np.float32)
    smoothing_factors = np.linspace(0.75, 0.35, NUM_BARS)

    smoothed_osc = np.zeros(NUM_OSC_POINTS, dtype=np.float32)

    def current_sample_idx() -> int:
        idx = int(audio.cursor)
        if idx < 0:
            idx = 0
        if idx >= len(samples):
            if audio.looping:
                idx = idx % len(samples)
            else:
                idx = len(samples) - 1
                exit()
        return idx

    running = True
    t = 0.0
    yaw = -90.0
    is_captured = False

    started = False

    while running:
        for event in engine.poll_events():
            if event.name == "Quit":
                engine.quit()
                exit()

        frame_start = time.time()
        t += DT

        if engine.is_key_down("P"):
            audio.play()
            started = True

        if engine.is_key_down("Tab"):
            is_captured = not is_captured
            engine.set_mouse_capture(is_captured)

        if is_captured:
            rel = engine.get_mouse_rel()
            if rel[0] != 0 or rel[1] != 0:
                sens = 0.05
                engine.rotate_camera(rel[0] * sens, -rel[1] * sens)
                yaw += rel[0] * sens

        rad = math.radians(yaw)
        fx = math.cos(rad)
        fz = math.sin(rad)
        rx_dir = -fz
        rz_dir = fx
        move_speed = 5.0 * DT

        if engine.is_key_down("W"):
            engine.move_camera(fx * move_speed, 0.0, fz * move_speed)
        if engine.is_key_down("S"):
            engine.move_camera(-fx * move_speed, 0.0, -fz * move_speed)
        if engine.is_key_down("A"):
            engine.move_camera(-rx_dir * move_speed, 0.0, -rz_dir * move_speed)
        if engine.is_key_down("D"):
            engine.move_camera(rx_dir * move_speed, 0.0, rz_dir * move_speed)

        if engine.is_key_down("Space"):
            engine.move_camera(0.0, move_speed, 0.0)
        if engine.is_key_down("Left Shift"):
            engine.move_camera(0.0, -move_speed, 0.0)
        if engine.is_key_down("Escape"):
            engine.quit()
            exit()

        if started:
            cursor = current_sample_idx()
            half = fft_size // 2
            start = cursor - half
            end = start + fft_size
            if start < 0 or end > len(samples):
                buf = np.zeros(fft_size, dtype=np.float32)
                s = max(0, start)
                e = min(len(samples), end)
                insert_from = s - start
                insert_to = insert_from + (e - s)
                if e > s:
                    buf[insert_from:insert_to] = samples[s:e]
            else:
                buf = samples[start:end].astype(np.float32)

            buf *= win
            fft_res = np.fft.rfft(buf)
            mags = np.abs(fft_res)

            freqs = np.fft.rfftfreq(fft_size, d=1.0 / sr)
            interp_func = interp.interp1d(
                freqs, mags, kind="cubic", bounds_error=False, fill_value=0
            )
            bands = interp_func(band_freqs)
            bands = np.maximum(bands, 0)
            eps = 1e-8
            bands_db = 20.0 * np.log10(bands + eps)
            max_db = bands_db.max() if bands_db.size else 0.0
            bands_db -= max_db
            bands_norm = np.clip((bands_db + 40.0) / 40.0, 0.0, 1.0)

            smoothed = smoothed * smoothing_factors + bands_norm * (
                1.0 - smoothing_factors
            )

            for i, e in enumerate(bars):
                v = float(smoothed[i])
                height = MIN_BAR_HEIGHT + v * SCALE_FACTOR
                e.scale = (e.scale[0], height, e.scale[2])
                x = e.position[0]
                e.position = (x, height * 0.5 + BARS_Y_OFFSET, 0.0)

                vv = clamp(v, 0.0, 1.0)
                # base gradient from green (vv=0) to red (vv=1)
                base_r = vv
                base_g = 1.0 - vv
                base_b = 0.0
                pastel_strength = 0.1
                pastel_add = (0.96, 0.96, 0.94)
                r_f = base_r * (1.0 - pastel_strength) + pastel_add[0] * pastel_strength
                g_f = base_g * (1.0 - pastel_strength) + pastel_add[1] * pastel_strength
                b_f = base_b * (1.0 - pastel_strength) + pastel_add[2] * pastel_strength
                r = int(clamp(r_f, 0.0, 1.0) * 255)
                g = int(clamp(g_f, 0.0, 1.0) * 255)
                b = int(clamp(b_f, 0.0, 1.0) * 255)
                e.mesh.set_texture(Texture.from_color(r, g, b, 255))

            # compute bass energy
            bass_mags = mags[bass_bin_mask]
            if bass_mags.size > 0:
                bass_db = 20.0 * np.log10(bass_mags + 1e-9)
                bass_db_mean = float(np.mean(bass_db))
                bass_norm = clamp((bass_db_mean + 60.0) / 60.0, 0.0, 1.0)
            else:
                bass_norm = 0.0

            overall_energy = float(smoothed.mean()) if smoothed.size else 0.0
            brightness = clamp(0.3 + overall_energy * 0.9 + bass_norm * 0.4, 0.05, 1.5)
            directional_light.color = (brightness, brightness, brightness)

            # OSC smoothing & positions
            osc_half = OSC_WINDOW // 2
            osc_start = cursor - osc_half
            osc_end = osc_start + OSC_WINDOW
            if osc_start < 0 or osc_end > len(samples):
                osc_buf = np.zeros(OSC_WINDOW, dtype=np.float32)
                s = max(0, osc_start)
                e = min(len(samples), osc_end)
                insert_from = s - osc_start
                insert_to = insert_from + (e - s)
                if e > s:
                    osc_buf[insert_from:insert_to] = samples[s:e]
            else:
                osc_buf = samples[osc_start:osc_end].astype(np.float32)

            if osc_buf.size == 0:
                osc_resampled = np.zeros(NUM_OSC_POINTS, dtype=np.float32)
            else:
                src_idx = np.arange(osc_buf.size)
                target_idx = np.linspace(0, osc_buf.size - 1, NUM_OSC_POINTS)
                osc_resampled = np.interp(target_idx, src_idx, osc_buf)

            eps = 1e-9
            peak = float(np.max(np.abs(osc_resampled))) + eps
            osc_norm = osc_resampled / peak
            smoothed_osc = smoothed_osc * OSC_SMOOTH + osc_norm * (1.0 - OSC_SMOOTH)

            for i, e in enumerate(osc_spheres):
                v = float(smoothed_osc[i])
                x = e.position[0]
                y = OSC_BASE_Y + v * OSC_AMPLITUDE
                e.position = (x, y, 0.0)

        running = engine.update()
        elapsed = time.time() - frame_start
        sleep_time = DT - elapsed
        if sleep_time > 0.0:
            time.sleep(sleep_time)


if __name__ == "__main__":
    main()
