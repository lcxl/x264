extern crate pkg_config;

fn main() {
    let version = std::env::var("DEP_X264_VERSION").ok().or_else(|| {
        pkg_config::Config::new()
            .cargo_metadata(false)
            .probe("x264")
            .ok()
            .map(|x264| x264.version)
    });

    let n: Option<u64> = version.as_ref().and_then(|v| {
        v.split('.')
            .nth(1)
            .and_then(|n| n.parse().ok())
    });

    if let Some(n) = n {
        if n >= 149 {
            println!("rustc-cfg=yuyv");
        }
    }
}
