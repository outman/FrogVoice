use std::io::Write;

/// Generate a small MP3 file with LAME and verify symphonia can probe/decode it.
#[test]
fn test_mp3_roundtrip() {
    // 1. Generate a small MP3 file using LAME
    let mut encoder = mp3lame_encoder::Builder::new()
        .expect("builder")
        .with_num_channels(2)
        .expect("ch")
        .with_sample_rate(44100)
        .expect("sr")
        .with_brate(mp3lame_encoder::Bitrate::Kbps192)
        .expect("br")
        .build()
        .expect("build");

    // Generate 0.5 seconds of a simple tone (44100/2 samples * 2 channels)
    let num_samples = 22050;
    let mut pcm = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / 44100.0;
        let sample = (440.0 * t * 2.0 * std::f32::consts::PI).sin() * 0.3;
        pcm.push(sample); // L
        pcm.push(sample); // R
    }

    let input = mp3lame_encoder::InterleavedPcm(&pcm);
    let buf_size = mp3lame_encoder::max_required_buffer_size(num_samples);
    let mut mp3_buf: Vec<u8> = Vec::with_capacity(buf_size);
    let n = encoder
        .encode(input, mp3_buf.spare_capacity_mut())
        .expect("encode");
    unsafe {
        mp3_buf.set_len(n);
    }
    let flush_n = encoder
        .flush::<mp3lame_encoder::FlushNoGap>(mp3_buf.spare_capacity_mut())
        .expect("flush");
    unsafe {
        mp3_buf.set_len(n + flush_n);
    }

    // Write to temp file
    let tmp = std::env::temp_dir().join("test_tone.mp3");
    std::fs::File::create(&tmp)
        .unwrap()
        .write_all(&mp3_buf)
        .unwrap();
    eprintln!("Wrote {} bytes to {:?}", mp3_buf.len(), tmp);

    // 2. Try to open with symphonia
    let file = std::fs::File::open(&tmp).unwrap();
    let mss =
        symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();
    hint.with_extension("mp3");

    let result = symphonia::default::get_probe().format(
        &hint,
        mss,
        &symphonia::core::formats::FormatOptions::default(),
        &symphonia::core::meta::MetadataOptions::default(),
    );

    match result {
        Ok(probed) => {
            eprintln!("SUCCESS! Tracks: {}", probed.format.tracks().len());
            for t in probed.format.tracks() {
                eprintln!(
                    "  Track {}: codec={:?}, rate={:?}, channels={:?}",
                    t.id, t.codec_params.codec, t.codec_params.sample_rate, t.codec_params.channels
                );
            }
        }
        Err(e) => {
            panic!("FAILED to probe MP3: {:?}", e);
        }
    }

    // Cleanup
    let _ = std::fs::remove_file(&tmp);
}

/// Test with a real-world MP3 file (with ID3 tags etc.)
#[test]
fn test_real_mp3_probe() {
    let real_mp3 = std::path::Path::new("/System/Library/PrivateFrameworks/PersonalAudio.framework/Versions/A/Resources/Enrollment_1+15dB.mp3");
    if !real_mp3.exists() {
        eprintln!("Skipping: real MP3 not found at {:?}", real_mp3);
        return;
    }

    eprintln!("Testing: {:?}", real_mp3);
    let header = std::fs::read(real_mp3).unwrap();
    eprintln!("File size: {} bytes, first 4 bytes: {:02x}{:02x}{:02x}{:02x}",
        header.len(), header[0], header[1], header[2], header[3]);

    let file = std::fs::File::open(real_mp3).expect("open real mp3");
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();
    hint.with_extension("mp3");

    let result = symphonia::default::get_probe().format(
        &hint,
        mss,
        &symphonia::core::formats::FormatOptions::default(),
        &symphonia::core::meta::MetadataOptions::default(),
    );

    match &result {
        Ok(probed) => {
            eprintln!("SUCCESS! Tracks: {}", probed.format.tracks().len());
            for t in probed.format.tracks() {
                eprintln!(
                    "  Track {}: codec={:?}, rate={:?}, channels={:?}",
                    t.id, t.codec_params.codec, t.codec_params.sample_rate, t.codec_params.channels
                );
            }
        }
        Err(e) => {
            eprintln!("FAILED to probe real MP3: {:?}", e);
        }
    }

    assert!(result.is_ok(), "symphonia should be able to probe a real MP3 file");
}
