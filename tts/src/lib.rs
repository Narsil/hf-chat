use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::parler_tts::{Config, Model};
use hf_hub::api::tokio::ApiError;
use log::info;
use std::{io::Write, path::PathBuf};
use tokenizers::Tokenizer;

mod bs1770;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Candle(#[from] candle_core::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Generic(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub async fn tts(input: &str, output: &PathBuf) -> Result<(), Error> {
    let mut audio: Vec<f32> = vec![];
    info!("Starting tts to {output:?}");
    let api = hf_hub::api::tokio::Api::new()?;
    let model_id = "parler-tts/parler-tts-large-v1".to_string();
    let revision = "main".to_string();
    let repo = api.repo(hf_hub::Repo::with_revision(
        model_id,
        hf_hub::RepoType::Model,
        revision,
    ));
    let model_files = hub_load_safetensors(&repo, "model.safetensors.index.json").await?;
    let config = repo.get("config.json").await?;
    let tokenizer = repo.get("tokenizer.json").await?;
    let tokenizer = Tokenizer::from_file(tokenizer)?;

    let device = Device::new_metal(0)?;
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&model_files, DType::F32, &device)? };
    let config: Config = serde_json::from_reader(std::fs::File::open(config)?)?;
    let mut model = Model::new(&config, vb)?;
    info!("Model loaded");
    for part in input.split(['.', '!', '?']) {
        info!("Chunk {part:?}");
        let description = "A female speaker delivers a slightly expressive and animated speech with a moderate speed and pitch. The recording is of very high quality, with the speaker's voice sounding clear and very close up.";

        let seed = 0;
        let max_steps = 512;
        let description_tokens = tokenizer.encode(description, true)?.get_ids().to_vec();
        let description_tokens = Tensor::new(description_tokens, &device)?.unsqueeze(0)?;
        let prompt_tokens = tokenizer.encode(part, true)?.get_ids().to_vec();
        let prompt_tokens = Tensor::new(prompt_tokens, &device)?.unsqueeze(0)?;
        let lp = candle_transformers::generation::LogitsProcessor::new(seed, None, None);
        info!("Encoded");
        let codes = model.generate(&prompt_tokens, &description_tokens, lp, max_steps)?;
        info!("Codes generated");
        let codes = codes.to_dtype(DType::I64)?;
        // codes.save_safetensors("codes", "out.safetensors")?;
        let codes = codes.unsqueeze(0)?;
        let pcm = model
            .audio_encoder
            .decode_codes(&codes.to_device(&device)?)?;
        info!("Output pcm");
        let pcm = pcm.i((0, 0))?;
        let pcm = normalize_loudness(&pcm, 24_000, true)?;
        let pcm = pcm.to_vec1::<f32>()?;
        audio.extend(&pcm);
    }
    if let Some(p) = output.parent() {
        std::fs::create_dir_all(p)?
    };
    let mut output = std::fs::File::create(output)?;
    write_pcm_as_ogg(&mut output, &audio, config.audio_encoder.sampling_rate)?;
    info!("Wrote audio file into {output:?}");
    Ok(())
}

pub trait Sample {
    fn to_i16(&self) -> i16;
}

impl Sample for f32 {
    fn to_i16(&self) -> i16 {
        (self.clamp(-1.0, 1.0) * 32767.0) as i16
    }
}

impl Sample for f64 {
    fn to_i16(&self) -> i16 {
        (self.clamp(-1.0, 1.0) * 32767.0) as i16
    }
}

impl Sample for i16 {
    fn to_i16(&self) -> i16 {
        *self
    }
}

pub fn write_pcm_as_ogg<W: Write, S: Sample>(
    w: &mut W,
    samples: &[S],
    sample_rate: u32,
) -> Result<(), Error> {
    let len = 12u32; // header
    let len = len + 24u32; // fmt
    let len = len + samples.len() as u32 * 2 + 8; // data
    let n_channels = 1u16;
    let bytes_per_second = sample_rate * 2 * n_channels as u32;
    w.write_all(b"RIFF")?;
    w.write_all(&(len - 8).to_le_bytes())?; // total length minus 8 bytes
    w.write_all(b"WAVE")?;

    // Format block
    w.write_all(b"fmt ")?;
    w.write_all(&16u32.to_le_bytes())?; // block len minus 8 bytes
    w.write_all(&1u16.to_le_bytes())?; // PCM
    w.write_all(&n_channels.to_le_bytes())?; // one channel
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&bytes_per_second.to_le_bytes())?;
    w.write_all(&2u16.to_le_bytes())?; // 2 bytes of data per sample
    w.write_all(&16u16.to_le_bytes())?; // bits per sample

    // Data block
    w.write_all(b"data")?;
    w.write_all(&(samples.len() as u32 * 2).to_le_bytes())?;
    for sample in samples.iter() {
        w.write_all(&sample.to_i16().to_le_bytes())?
    }
    Ok(())
}

/// Loads the safetensors files for a model from the hub based on a json index file.
pub async fn hub_load_safetensors(
    repo: &hf_hub::api::tokio::ApiRepo,
    json_file: &str,
) -> Result<Vec<std::path::PathBuf>, Error> {
    let json_file = repo.get(json_file).await?;
    let json_file = std::fs::File::open(json_file)?;
    let json: serde_json::Value = serde_json::from_reader(&json_file)?;
    let weight_map = match json.get("weight_map") {
        None => panic!("no weight map in {json_file:?}"),
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => panic!("weight map in {json_file:?} is not a map"),
    };
    let mut safetensors_files = std::collections::HashSet::new();
    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file.to_string());
        }
    }
    let safetensors_files = futures::future::join_all(
        safetensors_files
            .iter()
            .map(|v| async { repo.get(v).await }),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(safetensors_files)
}

// https://github.com/facebookresearch/audiocraft/blob/69fea8b290ad1b4b40d28f92d1dfc0ab01dbab85/audiocraft/data/audio_utils.py#L57
pub fn normalize_loudness(
    wav: &Tensor,
    sample_rate: u32,
    loudness_compressor: bool,
) -> Result<Tensor, Error> {
    let energy = wav.sqr()?.mean_all()?.sqrt()?.to_vec0::<f32>()?;
    if energy < 2e-3 {
        return Ok(wav.clone());
    }
    let wav_array = wav.to_vec1::<f32>()?;
    let mut meter = crate::bs1770::ChannelLoudnessMeter::new(sample_rate);
    meter.push(wav_array.into_iter());
    let power = meter.as_100ms_windows();
    let loudness = match crate::bs1770::gated_mean(power) {
        None => return Ok(wav.clone()),
        Some(gp) => gp.loudness_lkfs() as f64,
    };
    let delta_loudness = -14. - loudness;
    let gain = 10f64.powf(delta_loudness / 20.);
    let wav = (wav * gain)?;
    if loudness_compressor {
        Ok(wav.tanh()?)
    } else {
        Ok(wav)
    }
}
