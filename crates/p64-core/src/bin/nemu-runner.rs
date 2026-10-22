use p64_core::{
    recompiler::{Backend, Engine},
    run_game, N64,
};
use std::time::Instant;

const USAGE: &str = "Usage: nemu-runner ROM.z64 [--backend interpreter|jit|aot] [--aot FILE] [--stop-on-completion] [--unthrottled]";
const WORKER_STACK_BYTES: usize = 16 * 1024 * 1024;
const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 720;

#[path = "nemu-runner/options.rs"]
mod options;
use options::Options;

#[cfg(feature = "accuracy-diagnostics")]
#[path = "nemu-runner/diagnostics.rs"]
mod diagnostics;

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Interpreter => "interpreter",
        Backend::Jit => "jit",
        Backend::Aot => "aot",
    }
}

fn run(options: Options) -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let mut device = N64::with_ui(p64_gui::ui::Ui::try_new()?);
    device.is_viewer.stop_on_completion = options.stop_on_completion;
    device.video.host.host_throttle = !options.unthrottled;
    p64_gui::video::set_unthrottled(options.unthrottled);
    eprintln!(
        "Host pacing={}",
        if options.unthrottled {
            "unthrottled (VI sleep and renderer VSync disabled)"
        } else {
            "normal"
        }
    );
    let preparation_started = Instant::now();
    device.execution_engine = options.create_engine()?;
    eprintln!(
        "Backend preparation seconds={:.3}",
        preparation_started.elapsed().as_secs_f64()
    );
    let started = Instant::now();
    run_game(
        &options.rom,
        &mut device,
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        false,
    )?;
    report_results(&device, &options, started);

    Ok(())
}

fn report_results(device: &N64, options: &Options, started: Instant) {
    let backend = backend_name(options.backend);
    let unthrottled = options.unthrottled;
    eprintln!("Emulator stopped at PC={:#018x}", device.cpu.execution.pc);
    eprintln!(
        "Backend={backend} unthrottled={unthrottled} native={} fallback={} compiled={} blocks={} block_calls={} branch_calls={} memory_calls={} wall_seconds={:.3}",
        device.execution_engine.stats.native_executions,
        device.execution_engine.stats.fallback_executions,
        device.execution_engine.compiled_instructions(),
        device.execution_engine.compiled_blocks(),
        device.execution_engine.stats.native_block_executions,
        device.execution_engine.stats.native_branch_executions,
        device.execution_engine.stats.native_memory_executions,
        started.elapsed().as_secs_f64()
    );
    #[cfg(feature = "accuracy-diagnostics")]
    diagnostics::report(device);
}

fn main() {
    let result = Options::parse(std::env::args_os().skip(1)).and_then(|options| {
        std::thread::Builder::new()
            .name("nemu-runner".into())
            .stack_size(WORKER_STACK_BYTES)
            .spawn(move || run(options))
            .map_err(|error| format!("Start emulator thread: {error}"))?
            .join()
            .map_err(|_| "Emulator thread panicked".to_owned())?
    });

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
