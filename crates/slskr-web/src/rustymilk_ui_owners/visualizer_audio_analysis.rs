use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) struct RustyMilkAudioAnalyzer {
    pub(super) _context: web_sys::AudioContext,
    pub(super) analyser: web_sys::AnalyserNode,
    pub(super) frequency_bins: RefCell<Vec<u8>>,
    pub(super) _source: web_sys::MediaElementAudioSourceNode,
    pub(super) waveform_bins: RefCell<Vec<u8>>,
}

#[cfg(target_arch = "wasm32")]
impl RustyMilkAudioAnalyzer {
    pub(super) fn new(audio: &web_sys::HtmlAudioElement) -> Result<Self, JsValue> {
        let context = web_sys::AudioContext::new()?;
        let source = context.create_media_element_source(audio)?;
        let analyser = context.create_analyser()?;
        analyser.set_fft_size(1024);
        source.connect_with_audio_node(&analyser)?;
        analyser.connect_with_audio_node(&context.destination())?;
        let frequency_bins = RefCell::new(vec![0; analyser.frequency_bin_count() as usize]);
        let waveform_bins = RefCell::new(vec![0; analyser.fft_size() as usize]);
        Ok(Self {
            _context: context,
            analyser,
            frequency_bins,
            _source: source,
            waveform_bins,
        })
    }

    pub(super) fn snapshot(&self, time: f64) -> RustyMilkAudioSnapshot {
        let mut frequency_bins = self.frequency_bins.borrow_mut();
        self.analyser.get_byte_frequency_data(&mut frequency_bins);
        let length = frequency_bins.len();
        if length == 0 {
            return RustyMilkAudioSnapshot::synthetic(time);
        }
        let band = |start: usize, end: usize| -> f64 {
            let end = end.min(length).max(start + 1);
            let mut total = 0.0;
            let mut count = 0.0;
            for index in start..end {
                total += frequency_bins[index] as f64 / 255.0;
                count += 1.0;
            }
            if count == 0.0 {
                0.0
            } else {
                total / count
            }
        };
        let spectrum = frequency_bins
            .iter()
            .map(|value| *value as f64 / 255.0)
            .collect::<Vec<_>>();
        let bands = RustyMilkAudioBands {
            bass: band(0, length / 8),
            mid: band(length / 8, length / 3),
            treble: band(length / 3, length),
            source: "audio",
        };
        drop(frequency_bins);
        let mut waveform_bins = self.waveform_bins.borrow_mut();
        self.analyser.get_byte_time_domain_data(&mut waveform_bins);
        let waveform = waveform_bins
            .iter()
            .map(|value| (*value as f64 - 128.0) / 128.0)
            .collect::<Vec<_>>();
        RustyMilkAudioSnapshot {
            bands,
            source: "audio",
            spectrum,
            waveform,
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
pub(super) struct RustyMilkAudioSnapshot {
    pub(super) bands: RustyMilkAudioBands,
    pub(super) source: &'static str,
    pub(super) spectrum: Vec<f64>,
    pub(super) waveform: Vec<f64>,
}

#[cfg(target_arch = "wasm32")]
impl RustyMilkAudioSnapshot {
    pub(super) fn synthetic(time: f64) -> Self {
        Self {
            bands: RustyMilkAudioBands::synthetic(time),
            source: "synthetic",
            spectrum: Vec::new(),
            waveform: Vec::new(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RustyMilkAudioBands {
    pub(super) bass: f64,
    pub(super) mid: f64,
    pub(super) treble: f64,
    pub(super) source: &'static str,
}

#[cfg(target_arch = "wasm32")]
impl RustyMilkAudioBands {
    pub(super) fn synthetic(time: f64) -> Self {
        Self {
            bass: (time * 1.9).sin() * 0.5 + 0.5,
            mid: (time * 1.17 + 1.3).sin() * 0.5 + 0.5,
            treble: (time * 2.7 + 0.4).sin() * 0.5 + 0.5,
            source: "synthetic",
        }
    }
}
