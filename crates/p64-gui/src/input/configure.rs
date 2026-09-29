use super::capture::{CaptureResult, ProfileCapture};
use super::Input;
use crate::ui;
use rusttype::{point, Font, Scale};
use sdl2::{pixels::Color, rect::Point, render::Canvas, video::Window};
use std::time::{Duration, Instant};

const CAPTURE_COOLDOWN: Duration = Duration::from_millis(300);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(100);

const KEY_LABELS: &[(&str, Input)] = &[
    ("A", Input::AButton),
    ("B", Input::BButton),
    ("Start", Input::StartButton),
    ("D Up", Input::UDPad),
    ("D Down", Input::DDPad),
    ("D Left", Input::LDPad),
    ("D Right", Input::RDPad),
    ("C Up", Input::UCButton),
    ("C Down", Input::DCButton),
    ("C Left", Input::LCButton),
    ("C Right", Input::RCButton),
    ("L", Input::LTrig),
    ("R", Input::RTrig),
    ("Z", Input::ZTrig),
    ("Control Stick Up", Input::AxisUp),
    ("Control Stick Down", Input::AxisDown),
    ("Control Stick Left", Input::AxisLeft),
    ("Control Stick Right", Input::AxisRight),
];

pub fn draw_text(text: &str, canvas: &mut Canvas<Window>, font: &Font) {
    let scale = Scale::uniform(32.0);
    let v_metrics = font.v_metrics(scale);

    let text_width = font
        .layout(text, scale, point(0.0, 0.0))
        .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
        .last()
        .unwrap_or(0.0);

    let text_height = v_metrics.ascent - v_metrics.descent;

    let x = (960.0 - text_width) / 3.0;
    let y = (720.0 - text_height) / 3.0;

    let offset = point(x, y + v_metrics.ascent);

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    font.layout(text, scale, offset).for_each(|glyph| {
        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                if v > 0.5 {
                    canvas.set_draw_color(Color::RGB(255, 255, 255));
                    let _ = canvas.draw_point(Point::new(x as i32 + bb.min.x, y as i32 + bb.min.y));
                }
            });
        }
    });

    canvas.present();
}

pub fn configure_profile(ui: &mut ui::Ui, profile: String) -> Result<bool, String> {
    if profile == "Default" || profile.is_empty() {
        return Err("Profile name must be nonempty and cannot be Default".to_owned());
    }

    let joystick_subsystem = ui.sdl_context.joystick()?;
    let _joysticks: Vec<_> = (0..joystick_subsystem.num_joysticks()?)
        .map(|i| joystick_subsystem.open(i))
        .collect();

    let window = ui
        .sdl_context
        .video_subsystem
        .as_ref()
        .ok_or_else(|| "SDL video subsystem unavailable".to_owned())?
        .window("configure input profile", 640, 480)
        .position_centered()
        .build()
        .map_err(|error| error.to_string())?;
    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|error| error.to_string())?;

    let mut capture = ProfileCapture::default();
    let mut event_pump = ui.sdl_context.event_pump()?;

    let roboto = Font::try_from_bytes(include_bytes!("../Roboto-Regular.ttf"))
        .ok_or_else(|| "Input configuration font is invalid".to_owned())?;

    for &(key, value) in KEY_LABELS {
        event_pump.poll_iter().for_each(|_| {});

        draw_text(
            &format!("Press the button to bind to: {key}"),
            &mut canvas,
            &roboto,
        );

        let mut key_set = false;
        let mut last_input_time = Instant::now();

        while !key_set {
            std::thread::sleep(CAPTURE_POLL_INTERVAL);

            if last_input_time.elapsed() >= CAPTURE_COOLDOWN {
                for event in event_pump.poll_iter() {
                    match capture.capture(value, event) {
                        CaptureResult::Closed => return Ok(false),
                        CaptureResult::Ignored => {}
                        CaptureResult::Bound => {
                            key_set = true;
                            last_input_time = Instant::now();
                        }
                    }
                }
            }
        }
    }

    let new_profile = capture.into_profile();
    ui.input_config_mut()
        .input_profiles
        .insert(profile, new_profile);
    Ok(true)
}
