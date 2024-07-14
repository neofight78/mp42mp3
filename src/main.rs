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
            ffmpeg::ChannelLayout::STEREO,
            audio_decoder.rate(),
        )
        .unwrap();

    let mut input_frame = ffmpeg::util::frame::Audio::empty();
    let mut output_frame = ffmpeg::util::frame::Audio::empty();

    let mut audio_samples: [Vec<f32>; 2] = [Vec::new(), Vec::new()];

    for (stream, packet) in input.packets() {
        if stream.id() == audio_stream_id {
            audio_decoder.send_packet(&packet).unwrap();
            while audio_decoder.receive_frame(&mut input_frame).is_ok() {
                audio_resampler
                    .run(&input_frame, &mut output_frame)
                    .unwrap();

                audio_samples[0].extend_from_slice(&output_frame.plane::<f32>(0));
                audio_samples[1].extend_from_slice(&output_frame.plane::<f32>(1));
            }
        }
    }

    println!("Decoded audio!");

    println!("Exporting audio...");

    let mut output = ffmpeg::format::output("test.mp3").unwrap();

    let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::MP3).unwrap();
    let mut stream = output.add_stream(codec).unwrap();

    let context = ffmpeg::codec::context::Context::new();
    let mut encoder = context.encoder().audio().unwrap();
    encoder.set_rate(audio_decoder.rate() as i32);
    encoder.set_channel_layout(ffmpeg::ChannelLayout::STEREO);
    encoder.set_channels(2);
    encoder.set_bit_rate(128_000);
    encoder.set_format(ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar));

    let mut encoder = encoder.open_as(codec).unwrap();

    stream.set_parameters(&encoder);

    output.write_header().unwrap();

    let frame_size = 1152;
    let mut samples_output = 0;

    for chunk in audio_samples[0].chunks(frame_size).zip(audio_samples[1].chunks(frame_size)) {
        let mut frame = ffmpeg::util::frame::Audio::new(
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar),
            frame_size,
            ffmpeg::ChannelLayout::STEREO,
        );
        frame.plane_mut(0).copy_from_slice(&chunk.0);
        frame.plane_mut(1).copy_from_slice(&chunk.1);
        frame.set_pts(Some(samples_output as i64));

        encoder.send_frame(&frame).unwrap();

        let mut packet = ffmpeg::Packet::empty();
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(0);
            packet.write(&mut output).unwrap();
        }

        samples_output += chunk.0.len();
    }

    encoder.send_eof().unwrap();

    let mut packet = ffmpeg::Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        packet.set_stream(0);
        packet.write(&mut output).unwrap();
    }

    output.write_trailer().unwrap();

    println!("Audio exported!");
}
