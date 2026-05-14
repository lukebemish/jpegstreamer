use std::error::Error;
use std::io::{self, BufRead, Write};
use clap::Parser;

#[derive(Debug, Clone)]
struct Crop {
    w: usize,
    h: usize,
    x: usize,
    y: usize
}

#[derive(Parser, Debug)]
#[command(about = "Modify JPEG images while streaming", long_about = None)]
#[command(version = if let Some(version) = option_env!("PACKAGE_VERSION") { version} else { env!("CARGO_PKG_VERSION") })]
struct Args {
    /// Crop the image to the supplied bounds
    #[arg(short, long, value_name = "W:H:X:Y", value_parser=parse_crop_string)]
    crop: Crop
}

fn parse_crop_string(s: &str) -> Result<Crop, Box<dyn Error + Send + Sync>> {
    let mut w: [usize; 4] = [0, 0, 0, 0];
    let mut count = 0;
    for (i, part) in s.split(':').enumerate() {
        if i >= 4  {
            return Err("Too many components for crop: expected W:H:X:Y".into());
        }
        w[i] = part.parse()?;
        count += 1;
    }
    if count < 4  {
        return Err("Too few components for crop: expected W:H:X:Y".into());
    }
    return Ok(Crop {
        w: w[0],
        h: w[1],
        x: w[2],
        y: w[3]
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut buffer = vec![];
    let delim = b"\xFF\xD9";
    let lastbyte = delim[delim.len()-1];
    let soi = b"\xFF\xD8";

    let mut transform = turbojpeg::Transform::default();
    transform.crop = Some(turbojpeg::TransformCrop {
        x: args.crop.x,
        y: args.crop.y,
        width: Some(args.crop.w),
        height: Some(args.crop.h)
    });

    let mut stdout = io::stdout();

    loop {
        let bytes_read = reader.read_until(lastbyte, &mut buffer)?;
        if bytes_read == 0 {
            break; // End of file
        }

        if buffer.ends_with(delim) {
            // ffmpeg's stream from an MJPEG seems to include duplicate SOI tokens at the start of some images
            // Luckily, the SOI (like the EOI) will occur uniquely within an image's bytes
            let mut slice: &[u8] = buffer.as_ref();
            if let Some(location) = buffer
                .windows(soi.len())
                .rposition(|window| window == soi) {
                    if location != 0 {
                        slice = &buffer[location..];
                    }
                } else {
                    return Err("Could not find valid SOI".into())
                }
            
            let transformed = turbojpeg::transform(&transform, &slice)?;
            stdout.write_all(&transformed.as_ref())?;
            stdout.flush()?;

            buffer.clear();
        }
    }
    Ok(())
}
