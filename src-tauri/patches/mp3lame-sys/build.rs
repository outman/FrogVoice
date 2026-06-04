#![allow(clippy::single_match)]

const LAME_DIR: &str = "lame-3.100";

/// Build LAME using the `cc` crate — compiles individual .c files into a static library.
/// Used for native Windows builds AND cross-compiling to Windows from Unix.
fn build_with_cc() {
    const INCLUDE_MSVC: &str = ".include_msvc";

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let lame_dir = std::path::Path::new(LAME_DIR);
    let include_msvc = std::path::Path::new(&out_dir).join(INCLUDE_MSVC);
    let _ = std::fs::create_dir(&include_msvc);

    let target = std::env::var("TARGET").unwrap_or_default();
    let is_cross_compiling = cfg!(unix) && target.contains("windows");

    // When cross-compiling from Unix to Windows, configMS.h would define HAVE_XMMINTRIN_H
    // (because clang defines _M_X64 for MSVC targets). This enables SSE intrinsics that
    // clang can't resolve inline when cross-compiling. Generate a config.h that disables SSE.
    if is_cross_compiling {
        let config_content = std::fs::read_to_string(lame_dir.join("configMS.h"))
            .expect("Read configMS.h");
        // Remove the HAVE_XMMINTRIN_H define section to disable SSE intrinsics
        let patched = config_content.replace("#define HAVE_XMMINTRIN_H", "/* #undef HAVE_XMMINTRIN_H */");
        std::fs::write(include_msvc.join("config.h"), patched).expect("Write config.h");
    } else {
        std::fs::copy(lame_dir.join("configMS.h"), include_msvc.join("config.h")).expect("Copy config.h");
    }

    let mut cc = cc::Build::new();
    cc.warnings(false)
      .extra_warnings(false)
      .file(lame_dir.join("libmp3lame/bitstream.c"))
      .file(lame_dir.join("libmp3lame/encoder.c"))
      .file(lame_dir.join("libmp3lame/fft.c"))
      .file(lame_dir.join("libmp3lame/gain_analysis.c"))
      .file(lame_dir.join("libmp3lame/id3tag.c"))
      .file(lame_dir.join("libmp3lame/lame.c"))
      .file(lame_dir.join("libmp3lame/newmdct.c"))
      .file(lame_dir.join("libmp3lame/presets.c"))
      .file(lame_dir.join("libmp3lame/psymodel.c"))
      .file(lame_dir.join("libmp3lame/quantize_pvt.c"))
      // Skip SSE intrinsics when cross-compiling: clang targeting MSVC generates
      // function calls instead of inline instructions, causing linker errors.
      // The C fallback path is used instead — negligible performance impact.
      .file(lame_dir.join("libmp3lame/quantize.c"))
      .file(lame_dir.join("libmp3lame/reservoir.c"))
      .file(lame_dir.join("libmp3lame/set_get.c"))
      .file(lame_dir.join("libmp3lame/tables.c"))
      .file(lame_dir.join("libmp3lame/takehiro.c"))
      .file(lame_dir.join("libmp3lame/util.c"))
      .file(lame_dir.join("libmp3lame/vbrquantize.c"))
      .file(lame_dir.join("libmp3lame/VbrTag.c"))
      .file(lame_dir.join("libmp3lame/version.c"))
      .include(lame_dir.join("include"))
      .include(&include_msvc)
      .include(lame_dir.join("libmp3lame"))
      .define("TAKEHIRO_IEEE754_HACK", None)
      .define("FLOAT8", Some("float"))
      .define("REAL_IS_FLOAT", Some("1"))
      .define("BS_FORMAT", Some("BINARY"))
      .define("HAVE_CONFIG_H", None)
      .pic(false)
      .warnings(false);

    // Only include SSE-optimized file for native Windows builds (MSVC handles intrinsics correctly).
    // Cross-compiling from Unix with clang → MSVC target doesn't resolve SSE intrinsics inline.
    if cfg!(windows) || !is_cross_compiling {
        cc.file(lame_dir.join("libmp3lame/vector/xmm_quantize_sub.c"));
    }

    #[cfg(feature = "decoder")]
    {
        cc.define("HAVE_MPGLIB", None)
          .include(lame_dir.join("mpglib"))
          .file(lame_dir.join("mpglib/common.c"))
          .file(lame_dir.join("mpglib/dct64_i386.c"))
          .file(lame_dir.join("mpglib/decode_i386.c"))
          .file(lame_dir.join("mpglib/interface.c"))
          .file(lame_dir.join("mpglib/layer1.c"))
          .file(lame_dir.join("mpglib/layer2.c"))
          .file(lame_dir.join("mpglib/layer3.c"))
          .file(lame_dir.join("mpglib/tabinit.c"))
          .file(lame_dir.join("libmp3lame/mpglib_interface.c"));
    }

    if let Ok(compiler) = std::env::var("CC") {
        let compiler = std::path::Path::new(&compiler);
        let compiler = compiler.file_stem().expect("To have file name in CC").to_str().unwrap();
        match compiler {
            //because `cc` crate is retarded and cannot handle clang-cl correctly
            "clang-cl" => {
                cc.flag("/W0");
            },
            _ => (),
        }
    }

    cc.compile("mp3lame")
}

/// Build LAME using autotools (./configure && make).
/// Used on Unix targets (macOS, Linux) when NOT cross-compiling to Windows.
#[cfg(unix)]
fn build_unix_autotools() {
    let mut config = autotools::Config::new(LAME_DIR);

    #[cfg(feature = "decoder")]
    config.enable("decoder", None);
    #[cfg(not(feature = "decoder"))]
    config.disable("decoder", None);

    let host = std::env::var("HOST").expect("To have env:HOST");
    let target = std::env::var("TARGET").expect("To have env:TARGET");
    if let Ok(override_host) = std::env::var("MP3LAME_SYS_OVERRIDE_HOST") {
        config.config_option("host", Some(override_host.as_str()));
    } else if host != target {
        #[cfg(not(feature = "target_host"))]
        {
            if target.contains("android") {
                //Assume cross-compilation for android target
                config.config_option("host", Some(target.as_str()));
            } else if target.contains("apple") {
                if target.starts_with("aarch64") {
                    config.config_option("host", Some("arm-apple-darwin"));
                } else if target.starts_with("x86_64") {
                    config.config_option("host", Some("x86_64-apple-darwin"));
                } else {
                    println!("cargo:warning=Unsupported Apple target");
                }
            } else {
                println!("cargo:warning=Cross-compilation may not be supported");
            }
        }
        #[cfg(feature = "target_host")]
        {
            config.config_option("host", Some(target.as_str()));
        }
    }

    //Android cross compilation may require this flag
    if target.contains("android") || target.contains("ios") {
        config.cflag("-DSTDC_HEADERS");
    }

    let res = config.disable_shared()
                    .enable_static()
                    .disable("rpath", None)
                    .disable("frontend", None)
                    .disable("gtktest", None)
                    .with("pic", None)
                    .fast_build(true)
                    .build();

    //libraries are installed in <out>/lib
    println!("cargo:rustc-link-search=native={}/lib", res.display());
    println!("cargo:rustc-link-lib=static=mp3lame");
}

fn build() {
    let target = std::env::var("TARGET").unwrap_or_default();

    // On native Windows or cross-compiling TO Windows: use cc-based build.
    // Autotools cannot handle cross-compilation to Windows MSVC from Unix.
    if cfg!(windows) || target.contains("windows") {
        build_with_cc();
    } else {
        #[cfg(unix)]
        build_unix_autotools();
    }
}

fn main() {
    if std::env::var("DOCS_RS").map(|docs| docs == "1").unwrap_or(false) {
        //skip docs.rs build
        return;
    }

    build();
}
