use std::env;
use std::process::Command;
#[cfg(target_os = "windows")]
use winres::WindowsResource;

const COMMON_SDL_INCLUDE_PATHS: [&str; 2] = ["/usr/include/SDL2", "/usr/local/include/SDL2"];

const PARALLEL_RDP_PATH: &str = "parallel-rdp";
const STANDALONE_PATH: &str = "parallel-rdp/parallel-rdp-standalone";

const RDP_FILES: &[&str] = &[
    "command_ring.cpp",
    "rdp_device.cpp",
    "rdp_dump_write.cpp",
    "rdp_renderer.cpp",
    "video_interface.cpp",
];

const VULKAN_FILES: &[&str] = &[
    "buffer.cpp",
    "buffer_pool.cpp",
    "command_buffer.cpp",
    "command_pool.cpp",
    "context.cpp",
    "cookie.cpp",
    "descriptor_set.cpp",
    "device.cpp",
    "event_manager.cpp",
    "fence.cpp",
    "fence_manager.cpp",
    "image.cpp",
    "indirect_layout.cpp",
    "memory_allocator.cpp",
    "pipeline_event.cpp",
    "query_pool.cpp",
    "render_pass.cpp",
    "sampler.cpp",
    "semaphore.cpp",
    "semaphore_manager.cpp",
    "shader.cpp",
    "wsi.cpp",
];

const UTIL_FILES: &[&str] = &[
    "arena_allocator.cpp",
    "logging.cpp",
    "thread_id.cpp",
    "aligned_alloc.cpp",
    "timer.cpp",
    "timeline_trace_file.cpp",
    "environment.cpp",
    "thread_name.cpp",
];

const INCLUDE_PATHS: &[&str] = &[
    "parallel-rdp",
    "volk",
    "vulkan",
    "vulkan-headers/include",
    "util",
];

fn add_source_group(build: &mut cc::Build, directory: &str, files: &[&str]) {
    for file in files {
        build.file(format!("{STANDALONE_PATH}/{directory}/{file}"));
    }
}

fn add_source_files(build: &mut cc::Build) {
    add_source_group(build, "parallel-rdp", RDP_FILES);
    add_source_group(build, "vulkan", VULKAN_FILES);
    add_source_group(build, "util", UTIL_FILES);
    build
        .file(format!(
            "{STANDALONE_PATH}/vulkan/texture/texture_format.cpp"
        ))
        .file(format!("{STANDALONE_PATH}/volk/volk.c"))
        .file(format!("{PARALLEL_RDP_PATH}/bridge.cpp"));
}

fn set_includes(build: &mut cc::Build) {
    for path in INCLUDE_PATHS {
        build.include(format!("{STANDALONE_PATH}/{path}"));
    }
}

fn configure_bundled_sdl2(build: &mut cc::Build) -> bool {
    // Use target-specific headers from the Rust SDL dependency's link metadata.
    let Ok(include) = env::var("DEP_SDL2_INCLUDE") else {
        return false;
    };

    build.include(include);

    true
}

fn configure_sdl2(build: &mut cc::Build, target_os: &str) {
    if configure_bundled_sdl2(build) {
        return;
    }

    if target_os == "linux" {
        configure_linux_sdl2(build);
    } else {
        configure_sdl2_sdk(build);
    }
}

fn configure_sdl2_sdk(build: &mut cc::Build) {
    if let Ok(sdl2_dir) = env::var("SDL2_DIR") {
        println!("cargo:warning=SDL2_DIR: {}", sdl2_dir);

        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let (mingw_arch, msvc_arch) = match target_arch.as_str() {
            "aarch64" => ("aarch64", "arm64"),
            "x86" => ("i686", "x86"),
            "x86_64" => ("x86_64", "x64"),
            _ => panic!("SDL2_DIR needs a target-specific SDK for {target_arch}"),
        };

        let mingw_include = format!("{}/{mingw_arch}-w64-mingw32/include/SDL2", sdl2_dir);
        let mingw_lib = format!("{}/{mingw_arch}-w64-mingw32/lib", sdl2_dir);
        let msvc_include = format!("{}/include", sdl2_dir);
        let msvc_lib = format!("{}/lib/{msvc_arch}", sdl2_dir);

        if std::path::Path::new(&mingw_include).exists() {
            println!("cargo:warning=Using MinGW SDL2 include: {}", mingw_include);
            build.include(&mingw_include);
            println!("cargo:rustc-link-search=native={}", mingw_lib);
        } else if std::path::Path::new(&msvc_include).exists() {
            println!("cargo:warning=Using MSVC SDL2 include: {}", msvc_include);
            build.include(&msvc_include);
            println!("cargo:rustc-link-search=native={}", msvc_lib);
        } else {
            panic!("Could not find SDL2 include directory");
        }

        println!("cargo:rustc-link-lib=SDL2");
        println!("cargo:rustc-link-lib=SDL2main");
    } else {
        panic!("SDL2_DIR environment variable not set");
    }
}

fn configure_linux_sdl2(build: &mut cc::Build) {
    match Command::new("pkg-config")
        .args(["--cflags", "sdl2"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let cflags = String::from_utf8_lossy(&output.stdout);

            for flag in cflags.split_whitespace() {
                if flag.starts_with("-I") {
                    build.flag(flag);
                }
            }

            if let Ok(output) = Command::new("pkg-config").args(["--libs", "sdl2"]).output() {
                let libs = String::from_utf8_lossy(&output.stdout);

                for flag in libs.split_whitespace() {
                    if let Some(path) = flag.strip_prefix("-L") {
                        println!("cargo:rustc-link-search=native={path}");
                    } else if let Some(library) = flag.strip_prefix("-l") {
                        println!("cargo:rustc-link-lib={library}");
                    }
                }
            }

            return;
        }
        _ => {}
    }

    for path in &COMMON_SDL_INCLUDE_PATHS {
        if std::path::Path::new(path).exists() {
            println!("cargo:warning=Using SDL2 include path: {}", path);
            build.include(path);
            println!("cargo:rustc-link-lib=SDL2");
            return;
        }
    }

    println!("cargo:warning=Could not find SDL2 include paths automatically");

    build.include("/usr/include/SDL2");
    println!("cargo:rustc-link-lib=SDL2");
}

fn configure_msvc(build: &mut cc::Build) {
    build
        .flag("/EHsc")
        .flag("/bigobj")
        .flag("/permissive-")
        .flag("/Zc:preprocessor")
        .flag("/wd4244") // narrowing conversion
        .flag("/wd4267") // size_t narrowing
        .flag("/wd4389") // signed/unsigned comparison
        .flag("/W4")
        .flag("/O2")
        .define("WIN32", None)
        .define("_WINDOWS", None)
        .define("NOMINMAX", None)
        .define("_CRT_SECURE_NO_WARNINGS", None);
}

fn configure_platform_specific(build: &mut cc::Build, target_os: &str, target_env: &str) {
    // Build scripts run on HOST; platform defines must follow TARGET.
    match target_os {
        "windows" => {
            build.define("VK_USE_PLATFORM_WIN32_KHR", None);
        }
        "linux" => {
            build.define("VK_USE_PLATFORM_XCB_KHR", None);
            build.define("VK_USE_PLATFORM_XLIB_KHR", None);
        }
        "macos" => {
            build.define("VK_USE_PLATFORM_METAL_EXT", None);
        }
        _ => {}
    }

    if target_env != "msvc" {
        build
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-parameter");
    }
}

pub fn build_renderer() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Cargo target OS");
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").warnings(true).debug(true);

    if target_os == "windows" {
        #[cfg(target_os = "windows")]
        {
            let mut res = WindowsResource::new();
            res.set_icon("N64.ico");
            res.compile().unwrap();
        }

        if target_env == "msvc" {
            configure_msvc(&mut build);
        }
    }

    configure_sdl2(&mut build, &target_os);

    if let Ok(vulkan_sdk) = env::var("VULKAN_SDK") {
        let include = if target_os == "windows" {
            "Include"
        } else {
            "include"
        };

        build.include(format!("{vulkan_sdk}/{include}"));
    }

    add_source_files(&mut build);
    set_includes(&mut build);
    configure_platform_specific(&mut build, &target_os, &target_env);
    build.compile("parallel-rdp");
}
