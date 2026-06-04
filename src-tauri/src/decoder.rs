use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decoded PCM audio data in planar format (one Vec per channel).
pub struct PcmData {
    /// Planar f32 samples — samples[0] is channel 0, samples[1] is channel 1, etc.
    pub samples: Vec<Vec<f32>>,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_secs: f64,
}

/// Open an audio file with symphonia and return the FormatReader + selected track info.
fn open_audio(
    path: &str,
) -> Result<
    (
        Box<dyn symphonia::core::formats::FormatReader>,
        symphonia::core::formats::Track,
    ),
    String,
> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| format!("无法识别音频格式: {}", e))?;

    let format_reader = probed.format;

    // Find first audio track (has sample_rate — video tracks don't)
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| {
            t.codec_params.codec != CODEC_TYPE_NULL && t.codec_params.sample_rate.is_some()
        })
        .or_else(|| format_reader.default_track())
        .ok_or("未找到音频轨道")?
        .clone();

    Ok((format_reader, track))
}

/// Extract duration from track codec parameters.
fn duration_from_params(params: &symphonia::core::codecs::CodecParameters) -> Option<f64> {
    let tb = params.time_base?;
    let n_frames = params.n_frames?;
    let total_ts = params.start_ts + n_frames;
    let time = tb.calc_time(total_ts);
    Some(time.seconds as f64 + time.frac)
}

/// Probe the duration of an audio file using symphonia metadata.
/// Returns None if duration cannot be determined.
pub fn probe_duration(path: &str) -> Option<f64> {
    let (_format_reader, track) = open_audio(path).ok()?;
    duration_from_params(&track.codec_params)
}

/// Decode an audio file to planar f32 PCM data.
///
/// `progress_callback` is called with values 0.0..1.0 during decode.
pub fn decode_to_pcm<F: Fn(f64)>(path: &str, progress_callback: F) -> Result<PcmData, String> {
    let (mut format_reader, track) = open_audio(path)?;
    let params = &track.codec_params;

    let track_id = track.id;
    let sample_rate = params.sample_rate.ok_or("无法获取采样率")?;
    let channels = params.channels.ok_or("无法获取声道数")?;
    let num_channels = channels.count();

    let duration_secs = duration_from_params(params).unwrap_or(0.0);

    // Create decoder
    let decode_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(params, &decode_opts)
        .map_err(|e| format!("不支持该音频编码: {}", e))?;

    // Calculate total frames for progress reporting
    let total_frames = params.n_frames.unwrap_or(0) as f64;
    let mut decoded_frames: f64 = 0.0;

    // Decode all packets into planar f32 buffers
    let mut all_samples: Vec<Vec<f32>> = vec![Vec::new(); num_channels];

    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("解码错误: {}", e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let packet_frames = packet.dur() as u64;

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                let capacity = decoded.capacity() as u64;
                let mut sample_buf = SampleBuffer::<f32>::new(capacity, spec);
                sample_buf.copy_interleaved_ref(decoded);

                let interleaved = sample_buf.samples();
                // De-interleave: [L0, R0, L1, R1, ...] → [[L0, L1, ...], [R0, R1, ...]]
                for (i, sample) in interleaved.iter().enumerate() {
                    all_samples[i % num_channels].push(*sample);
                }

                decoded_frames += packet_frames as f64;
                if total_frames > 0.0 {
                    let progress = (decoded_frames / total_frames).min(1.0);
                    progress_callback(progress * 0.8);
                }
            }
            Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("解码错误: {}", e)),
        }
    }

    // Calculate actual duration from samples if metadata didn't provide it
    let actual_duration = if duration_secs > 0.0 {
        duration_secs
    } else if num_channels > 0 && !all_samples[0].is_empty() {
        all_samples[0].len() as f64 / sample_rate as f64
    } else {
        0.0
    };

    Ok(PcmData {
        samples: all_samples,
        sample_rate,
        channels: num_channels,
        duration_secs: actual_duration,
    })
}
