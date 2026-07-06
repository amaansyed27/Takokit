use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use takokit_core::{TakokitError, TakokitResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl Default for WavSpec {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
        }
    }
}

pub fn write_silence_wav(path: &Path, duration_ms: u32, spec: WavSpec) -> TakokitResult<u64> {
    let bytes_per_sample = u32::from(spec.bits_per_sample / 8);
    let sample_count = spec.sample_rate * duration_ms / 1_000;
    let data_len = sample_count * u32::from(spec.channels) * bytes_per_sample;
    let mut writer =
        BufWriter::new(File::create(path).map_err(|error| TakokitError::Audio(error.to_string()))?);

    writer.write_all(b"RIFF").map_err(io_error)?;
    writer
        .write_all(&(36 + data_len).to_le_bytes())
        .map_err(io_error)?;
    writer.write_all(b"WAVEfmt ").map_err(io_error)?;
    writer.write_all(&16u32.to_le_bytes()).map_err(io_error)?;
    writer.write_all(&1u16.to_le_bytes()).map_err(io_error)?;
    writer
        .write_all(&spec.channels.to_le_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&spec.sample_rate.to_le_bytes())
        .map_err(io_error)?;

    let byte_rate = spec.sample_rate * u32::from(spec.channels) * bytes_per_sample;
    let block_align = spec.channels * (spec.bits_per_sample / 8);
    writer
        .write_all(&byte_rate.to_le_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&block_align.to_le_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&spec.bits_per_sample.to_le_bytes())
        .map_err(io_error)?;
    writer.write_all(b"data").map_err(io_error)?;
    writer
        .write_all(&data_len.to_le_bytes())
        .map_err(io_error)?;
    writer
        .write_all(&vec![0; data_len as usize])
        .map_err(io_error)?;
    writer.flush().map_err(io_error)?;

    Ok(u64::from(44 + data_len))
}

fn io_error(error: std::io::Error) -> TakokitError {
    TakokitError::Audio(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_valid_riff_wave_header() {
        let path = std::env::temp_dir().join("takokit-silence-test.wav");
        let bytes = write_silence_wav(&path, 100, WavSpec::default()).expect("wav");
        let contents = fs::read(&path).expect("read wav");

        assert_eq!(&contents[0..4], b"RIFF");
        assert_eq!(&contents[8..12], b"WAVE");
        assert_eq!(bytes as usize, contents.len());

        let _ = fs::remove_file(path);
    }
}
