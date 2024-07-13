fn main() {
    ffmpeg::init().unwrap();

    println!("Decoding audio...");

    let mut input = ffmpeg::format::input("test.mp4").unwrap();

    let audio_stream = input.streams().best(ffmpeg::media::Type::Audio).unwrap();
    let audio_stream_id = audio_stream.id();

    let mut audio_decoder =
        ffmpeg::codec::context::Context::from_parameters(audio_stream.parameters())
            .unwrap()
            .decoder()
            .audio()
            .unwrap();

    let mut audio_resampler = audio_decoder
        .resampler(
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar),
            audio_decoder.channel_layout(),
            audio_decoder.rate(),
        )
        .unwrap();

    let mut input_frame = ffmpeg::util::frame::Audio::empty();
    let mut output_frame = ffmpeg::util::frame::Audio::empty();

    let mut audio_samples: Vec<f32> = Vec::new();

    for (stream, packet) in input.packets() {
        if stream.id() == audio_stream_id {
            audio_decoder.send_packet(&packet).unwrap();
            while audio_decoder.receive_frame(&mut input_frame).is_ok() {
                audio_resampler
                    .run(&input_frame, &mut output_frame)
                    .unwrap();

                let channel_count = output_frame.channels() as usize;
                let sample_count = output_frame.samples();
                let mut mixed_samples = vec![0.0; sample_count];

                for i in 0..channel_count {
                    let sample_data = output_frame.plane::<f32>(i);
                    for i in 0..sample_count {
                        mixed_samples[i] += sample_data[i] / channel_count as f32;
                    }
                }
                audio_samples.extend_from_slice(&mixed_samples);
            }
        }
    }

    println!("Decoded audio!");

    println!("Exporting audio...");

    let mut output = ffmpeg::format::output("test.mp3").unwrap();

    let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::MP3).unwrap();
    let stream = output.add_stream(codec).unwrap();

    let mut encoder = stream.codec().encoder().audio().unwrap();
    encoder.set_rate(audio_decoder.rate() as i32);
    encoder.set_channel_layout(ffmpeg::ChannelLayout::MONO);
    encoder.set_channels(1);
    encoder.set_bit_rate(128_000);
    encoder.set_format(ffmpeg::format::Sample::F32(
        ffmpeg::format::sample::Type::Planar,
    ));

    output.write_header().unwrap();

    let mut frame = ffmpeg::util::frame::Audio::new(
        ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar),
        audio_samples.len(),
        ffmpeg::ChannelLayout::MONO,
    );
    frame.plane_mut(0).copy_from_slice(audio_samples.as_slice());

    encoder.send_frame(&frame).unwrap();

    let mut packet = ffmpeg::Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        packet.set_stream(0);
        packet.write(&mut output).unwrap();
    }

    encoder.send_eof().unwrap();
    while encoder.receive_packet(&mut packet).is_ok() {
        packet.set_stream(0);
        packet.write(&mut output).unwrap();
    }

    output.write_trailer().unwrap();

    println!("Audio exported!");
}
