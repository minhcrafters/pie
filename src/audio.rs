use glam::Vec3;
use pyo3::prelude::*;
use resampler::{ResamplerFft, SampleRate};
use sdl2::audio::AudioCallback;
use std::sync::{Arc, Mutex};

pub enum AudioData {
    Clip { samples: Vec<f32>, channels: usize },
    Sine { freq: f32, phase: f32 },
}

#[pyclass]
pub struct AudioSource {
    pub position: Vec3,
    pub data: AudioData,
    #[pyo3(get, set)]
    pub looping: bool,
    #[pyo3(get)]
    pub playing: bool,
    pub cursor: usize,
    pub current_left_gain: f32,
    pub current_right_gain: f32,
    #[pyo3(get, set)]
    pub positional: bool,
}

impl AudioSource {
    pub fn new(position: Vec3, data: AudioData, looping: bool) -> Self {
        Self {
            position,
            data,
            looping,
            playing: false,
            cursor: 0,
            current_left_gain: 0.0,
            current_right_gain: 0.0,
            positional: true,
        }
    }
}

#[pymethods]
impl AudioSource {
    #[staticmethod]
    pub fn new_sine(freq: f32, looping: bool) -> Self {
        AudioSource::new(
            Vec3::new(0.0, 0.0, 0.0),
            AudioData::Sine { freq, phase: 0.0 },
            looping,
        )
    }

    #[staticmethod]
    pub fn new_clip(samples: Vec<f32>, looping: bool) -> Self {
        AudioSource::new(
            Vec3::new(0.0, 0.0, 0.0),
            AudioData::Clip {
                samples,
                channels: 1,
            },
            looping,
        )
    }

    #[staticmethod]
    pub fn from_wav(file: &str, looping: bool) -> PyResult<Self> {
        let reader = hound::WavReader::open(file).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to open WAV file: {}", e))
        })?;
        let spec = reader.spec();

        if spec.bits_per_sample != 16 || !(spec.channels == 1 || spec.channels == 2) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Unsupported WAV format: only 16-bit mono/stereo supported",
            ));
        }

        let raw: Vec<i16> = reader
            .into_samples::<i16>()
            .filter_map(Result::ok)
            .collect();

        let mut frames: Vec<f32> = Vec::with_capacity(raw.len());
        let ch = spec.channels as usize;
        for s in raw {
            frames.push(s as f32 / i16::MAX as f32);
        }

        let (samples, channels) = if spec.sample_rate != 44100 {
            let source_rate = SampleRate::try_from(spec.sample_rate).map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Unsupported sample rate: {}",
                    spec.sample_rate
                ))
            })?;
            let target_rate = SampleRate::Hz44100;

            let mut resampler = ResamplerFft::new(ch, source_rate, target_rate);
            let output_frames = resampler.chunk_size_output();
            let mut output = vec![0.0f32; output_frames];

            resampler
                .resample(&frames, &mut output)
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Unexpected error: {}", e))
                })
                .unwrap();

            (output, ch)
        } else {
            (frames, ch)
        };

        Ok(AudioSource::new(
            Vec3::new(0.0, 0.0, 0.0),
            AudioData::Clip { samples, channels },
            looping,
        ))
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    #[getter]
    pub fn get_position(&self) -> (f32, f32, f32) {
        (self.position.x, self.position.y, self.position.z)
    }

    #[setter]
    pub fn set_position(&mut self, position: (f32, f32, f32)) {
        self.position = Vec3::new(position.0, position.1, position.2);
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    #[getter]
    pub fn get_cursor(&self) -> usize {
        self.cursor
    }

    #[getter]
    pub fn get_duration(&self) -> f32 {
        match &self.data {
            AudioData::Clip { samples, channels } => {
                let ch = *channels;
                if ch == 0 {
                    0.0
                } else {
                    (samples.len() / ch) as f32 / 44100.0
                }
            }
            AudioData::Sine { .. } => f32::MAX,
        }
    }
}

pub struct ListenerState {
    pub position: Vec3,
    pub right: Vec3,
}

pub struct AudioMixer {
    pub sources: Arc<Mutex<Vec<Py<AudioSource>>>>,
    pub listener_state: Arc<Mutex<ListenerState>>,
}

impl AudioMixer {
    /// Create a new AudioMixer with the provided shared sources and listener state.
    /// Helper to construct the mixer.
    pub fn new(
        sources: Arc<Mutex<Vec<Py<AudioSource>>>>,
        listener_state: Arc<Mutex<ListenerState>>,
    ) -> Self {
        AudioMixer {
            sources,
            listener_state,
        }
    }
}

impl Default for AudioMixer {
    fn default() -> Self {
        AudioMixer {
            sources: Arc::new(Mutex::new(Vec::new())),
            listener_state: Arc::new(Mutex::new(ListenerState {
                position: Vec3::ZERO,
                right: Vec3::X,
            })),
        }
    }
}

impl AudioCallback for AudioMixer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = 0.0;
        }

        let (listener_pos, listener_right) = if let Ok(state) = self.listener_state.lock() {
            (state.position, state.right)
        } else {
            (Vec3::ZERO, Vec3::X)
        };

        Python::attach(|py| {
            if let Ok(mut sources) = self.sources.lock() {
                for source_py in sources.iter_mut() {
                    let mut source = match source_py.try_borrow_mut(py) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    if !source.playing {
                        continue;
                    }

                    let (target_left, target_right) = if !source.positional {
                        (1.0f32, 1.0f32)
                    } else {
                        let to_source = source.position - listener_pos;
                        let dist_sq = to_source.length_squared();
                        let dist = dist_sq.sqrt();

                        let direction = if dist > 0.001 {
                            to_source / dist
                        } else {
                            Vec3::Y
                        };

                        let gain = 1.0 / (dist_sq + 1.0);

                        let pan = direction.dot(listener_right);

                        let p = (pan.clamp(-1.0, 1.0) + 1.0) * 0.5;
                        let right_gain = p.sqrt();
                        let left_gain = (1.0_f32 - p).sqrt();

                        (left_gain * gain, right_gain * gain)
                    };

                    let samples_len = out.len() / 2;
                    let left_step = if samples_len > 0 {
                        (target_left - source.current_left_gain) / samples_len as f32
                    } else {
                        0.0
                    };
                    let right_step = if samples_len > 0 {
                        (target_right - source.current_right_gain) / samples_len as f32
                    } else {
                        0.0
                    };

                    let mut data = std::mem::replace(
                        &mut source.data,
                        AudioData::Sine {
                            freq: 0.0,
                            phase: 0.0,
                        },
                    );

                    match &mut data {
                        AudioData::Sine { freq, phase } => {
                            let freq_copy = *freq;
                            let mut phase_local = *phase;
                            let increment = freq_copy * 2.0 * std::f32::consts::PI / 44100.0;

                            for frame in out.chunks_mut(2) {
                                source.current_left_gain += left_step;
                                source.current_right_gain += right_step;

                                phase_local =
                                    (phase_local + increment) % (2.0 * std::f32::consts::PI);
                                let sample = phase_local.sin() * 0.5;

                                frame[0] += sample * source.current_left_gain;
                                frame[1] += sample * source.current_right_gain;
                            }

                            *phase = phase_local;
                        }
                        AudioData::Clip { samples, channels } => {
                            let ch = *channels;
                            let frames_available = if ch > 0 { samples.len() / ch } else { 0 };

                            for frame in out.chunks_mut(2) {
                                let next_frame_opt = if source.cursor < frames_available {
                                    Some(source.cursor)
                                } else if source.looping && frames_available > 0 {
                                    Some(0usize)
                                } else {
                                    None
                                };

                                source.current_left_gain += left_step;
                                source.current_right_gain += right_step;

                                if let Some(fidx) = next_frame_opt {
                                    if ch == 1 || source.positional {
                                        // mono playback (or positional sources use mono mix)
                                        let sample = if ch == 1 {
                                            samples[fidx]
                                        } else {
                                            // average stereo to mono for positional sources
                                            let l = samples[fidx * 2];
                                            let r = samples[fidx * 2 + 1];
                                            0.5 * (l + r)
                                        };

                                        frame[0] += sample * source.current_left_gain;
                                        frame[1] += sample * source.current_right_gain;
                                    } else {
                                        // non-positional stereo: keep channels separate
                                        let l = samples[fidx * 2];
                                        let r = samples[fidx * 2 + 1];
                                        frame[0] += l * source.current_left_gain;
                                        frame[1] += r * source.current_right_gain;
                                    }

                                    // advance cursor
                                    if source.cursor + 1 < frames_available {
                                        source.cursor += 1;
                                    } else if source.looping && frames_available > 0 {
                                        source.cursor = 0;
                                    } else {
                                        source.cursor = frames_available;
                                    }
                                } else {
                                    source.playing = false;
                                    break;
                                }
                            }
                        }
                    }

                    source.data = data;

                    source.current_left_gain = target_left;
                    source.current_right_gain = target_right;
                }
            }
        });

        // Recording support removed: do not forward mixed samples to any recorder.
    }
}
