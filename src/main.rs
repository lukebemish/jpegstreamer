use std::io::{self, BufRead, Error, Write};
use std::num::ParseIntError;
use std::str::FromStr;
use argparse::{ArgumentParser, Store};

struct Crop {
    w: usize,
    h: usize,
    x: usize,
    y: usize
}

#[derive(Debug)]
pub enum CommandError {
    Int(ParseIntError),
    IO(Error),
    TurboJpeg(turbojpeg::Error),
    Message(&'static str)
}

impl From<ParseIntError> for CommandError {
    fn from(e: ParseIntError) -> Self {
        CommandError::Int(e)
    }
}

impl From<Error> for CommandError {
    fn from(e: Error) -> Self {
        CommandError::IO(e)
    }
}

impl From<turbojpeg::Error> for CommandError {
    fn from(e: turbojpeg::Error) -> Self {
        CommandError::TurboJpeg(e)
    }
}

impl FromStr for Crop {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut w: [usize; 4] = [0, 0, 0, 0];
        let mut count = 0;
        for (i, part) in s.split(':').enumerate() {
            if i >= 4  {
                return Err(CommandError::Message("Too many components for crop"));
            }
            w[i] = part.parse()?;
            count += 1;
        }
        if count < 4  {
            return Err(CommandError::Message("Too few components for crop"));
        }
        return Ok(Crop {
            w: w[0],
            h: w[1],
            x: w[2],
            y: w[3]
        });
    }
}

fn main() -> Result<(), CommandError> {
    let mut crop: Crop = Crop { w: 0, h: 0, x: 0, y: 0 };
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Modify JPEG images while streaming.");
        ap.refer(&mut crop)
            .add_option(&["--crop"], Store,
            "Crop the image, as w:h:x:y")
            .required();
        ap.parse_args_or_exit();
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut buffer = vec![];
    let delim = b"\xFF\xD9";
    let lastbyte = delim[delim.len()-1];
    let soi = b"\xFF\xD8";

    let mut transform = turbojpeg::Transform::default();
    transform.crop = Some(turbojpeg::TransformCrop {
        x: crop.x,
        y: crop.y,
        width: Some(crop.w),
        height: Some(crop.h)
    });

    let mut stdout = io::stdout();

    loop {
        let bytes_read = reader.read_until(lastbyte, &mut buffer)?;
        if bytes_read == 0 {
            break; // End of file
        }

        if buffer.ends_with(delim) {
            if let Some(location) = buffer
                .windows(soi.len())
                .rposition(|window| window == soi) {
                    if location != 0 {
                        buffer.drain(..location);
                    }
                } else {
                    return Err(CommandError::Message("Could not find valid SOI"))
                }
            
            let transformed = turbojpeg::transform(&transform, &buffer)?;
            stdout.write_all(&transformed.as_ref())?;
            stdout.flush()?;

            buffer.clear();
        }
    }
    Ok(())
}
