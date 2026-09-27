use crate::ui;

pub fn try_set_frequency(ui: &mut ui::Ui, frequency: u64) -> Result<(), String> {
    let frequency = i32::try_from(frequency)
        .ok()
        .filter(|frequency| *frequency > 0)
        .ok_or_else(|| "Invalid audio frequency".to_owned())?;
    let desired_spec = sdl2::audio::AudioSpecDesired {
        freq: Some(frequency),
        channels: Some(crate::audio_format::CHANNEL_COUNT),
        samples: None,
    };
    let device = ui
        .sdl_context
        .audio_subsystem
        .as_ref()
        .ok_or_else(|| "SDL audio subsystem unavailable".to_owned())?
        .open_queue::<i16, _>(None, &desired_spec)?;
    device.resume();
    ui.window_audio.audio_device = Some(device);
    Ok(())
}

pub fn set_frequency(ui: &mut ui::Ui, frequency: u64) {
    if let Err(error) = try_set_frequency(ui, frequency) {
        tracing::error!("Unable to open audio queue: {error}");
    }
}
