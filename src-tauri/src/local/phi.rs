#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use crate::{Error, Generation, Query, Token};
use candle_transformers::models::mixformer::{Config, MixFormerSequentialForCausalLM as MixFormer};
use candle_transformers::models::quantized_mixformer::MixFormerSequentialForCausalLM as QMixFormer;

use candle::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;

enum Model {
    MixFormer(MixFormer),
    Quantized(QMixFormer),
}

struct TextGeneration {
    model: Model,
    device: Device,
    tokenizer: Tokenizer,
    logits_processor: LogitsProcessor,
    repeat_penalty: f32,
    repeat_last_n: usize,
}

impl TextGeneration {
    #[allow(clippy::too_many_arguments)]
    fn new(
        model: Model,
        tokenizer: Tokenizer,
        seed: u64,
        temp: Option<f64>,
        top_p: Option<f64>,
        repeat_penalty: f32,
        repeat_last_n: usize,
        device: &Device,
    ) -> Self {
        let logits_processor = LogitsProcessor::new(seed, temp, top_p);
        Self {
            model,
            tokenizer,
            logits_processor,
            repeat_penalty,
            repeat_last_n,
            device: device.clone(),
        }
    }

    fn run(&mut self, prompt: &str, sample_len: usize) -> Result<()> {
        use std::io::Write;
        println!("starting the inference loop");
        print!("{prompt}");
        std::io::stdout().flush()?;
        let mut tokens = self
            .tokenizer
            .encode(prompt, true)
            .map_err(E::msg)?
            .get_ids()
            .to_vec();

        let mut generated_tokens = 0usize;
        let eos_token = match self.tokenizer.get_vocab(true).get("<|endoftext|>") {
            Some(token) => *token,
            None => anyhow::bail!("cannot find the endoftext token"),
        };
        let start_gen = std::time::Instant::now();
        for index in 0..sample_len {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];
            let input = Tensor::new(ctxt, &self.device)?.unsqueeze(0)?;
            let logits = match &mut self.model {
                Model::MixFormer(m) => m.forward(&input)?,
                Model::Quantized(m) => m.forward(&input)?,
            };
            let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;
            let logits = if self.repeat_penalty == 1. {
                logits
            } else {
                let start_at = tokens.len().saturating_sub(self.repeat_last_n);
                candle_transformers::utils::apply_repeat_penalty(
                    &logits,
                    self.repeat_penalty,
                    &tokens[start_at..],
                )?
            };

            let next_token = self.logits_processor.sample(&logits)?;
            tokens.push(next_token);
            generated_tokens += 1;
            if next_token == eos_token {
                break;
            }
            let token = self.tokenizer.decode(&[next_token], true).map_err(E::msg)?;
            print!("{token}");
            std::io::stdout().flush()?;
        }
        let dt = start_gen.elapsed();
        println!(
            "\n{generated_tokens} tokens generated ({:.2} token/s)",
            generated_tokens as f64 / dt.as_secs_f64(),
        );
        Ok(())
    }
}

// #[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
// struct Args {
//     /// Run on CPU rather than on GPU.
//     #[arg(long)]
//     cpu: bool,
//
//     /// Enable tracing (generates a trace-timestamp.json file).
//     #[arg(long)]
//     tracing: bool,
//
//     #[arg(long)]
//     prompt: String,
//
//     /// The temperature used to generate samples.
//     #[arg(long)]
//     temperature: Option<f64>,
//
//     /// Nucleus sampling probability cutoff.
//     #[arg(long)]
//     top_p: Option<f64>,
//
//     /// The seed to use when generating random samples.
//     #[arg(long, default_value_t = 299792458)]
//     seed: u64,
//
//     /// The length of the sample to generate (in tokens).
//     #[arg(long, short = 'n', default_value_t = 100)]
//     sample_len: usize,
//
//     #[arg(long, default_value = "microsoft/phi-1_5")]
//     model_id: String,
//
//     #[arg(long, default_value = "refs/pr/18")]
//     revision: String,
//
//     #[arg(long)]
//     weight_file: Option<String>,
//
//     #[arg(long)]
//     quantized: bool,
//
//     /// Penalty to be applied for repeating tokens, 1. means no penalty.
//     #[arg(long, default_value_t = 1.1)]
//     repeat_penalty: f32,
//
//     /// The context size to consider for the repeat penalty.
//     #[arg(long, default_value_t = 64)]
//     repeat_last_n: usize,
// }

fn tokenizer() -> Result<Tokenizer, 

fn main() -> Result<()> {
    // use tracing_chrome::ChromeLayerBuilder;
    // use tracing_subscriber::prelude::*;

    // let args = Args::parse();
    let model_id = "microsoft/phi-1_5".to_string();
    let revision = "refs/pr/18".to_string();

    println!(
        "avx: {}, neon: {}, simd128: {}, f16c: {}",
        candle::utils::with_avx(),
        candle::utils::with_neon(),
        candle::utils::with_simd128(),
        candle::utils::with_f16c()
    );
    println!(
        "temp: {:.2} repeat-penalty: {:.2} repeat-last-n: {}",
        args.temperature.unwrap_or(0.),
        args.repeat_penalty,
        args.repeat_last_n
    );

    let start = std::time::Instant::now();
    let config = Config::v1_5();
    let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(&filename)?;
    let model = QMixFormer::new(&config, vb)?;
    let (model, device) = (Model::Quantized(model), Device::Cpu);
    // } else {
    //     let device = candle_examples::device(args.cpu)?;
    //     let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[filename], DType::F32, &device)? };
    //     let model = MixFormer::new(&config, vb)?;
    //     (Model::MixFormer(model), device)
    // };
    println!("loaded the model in {:?}", start.elapsed());

    let mut pipeline = TextGeneration::new(
        model,
        tokenizer,
        args.seed,
        args.temperature,
        args.top_p,
        args.repeat_penalty,
        args.repeat_last_n,
        &device,
    );
    pipeline.run(&args.prompt, args.sample_len)?;
    Ok(())
}

pub fn load_local(query: Query) -> Result<Pipeline, Error> {
    let tokenizer = tokenizer()?;
    let model = get_model()?;
    let encoded = tokenizer.encode(query.inputs.clone(), true)?;
    let tokens: Vec<u32> = encoded.get_ids().to_vec();
    let logits_processor = LogitsProcessor::new(
        0,
        Some(query.parameters.temperature as f64),
        Some(query.parameters.top_p as f64),
    );
    Ok(Pipeline {
        model,
        tokenizer,
        query,
        logits_processor,
        tokens: tokens.to_vec(),
    })
}
pub struct PipelineIter<'a> {
    pipeline: &'a mut Pipeline,
    tokens: Vec<u32>,
    all_tokens: Vec<u32>,
    i: usize,
    last: bool,
}

impl Pipeline {
    pub fn iter(&mut self) -> PipelineIter {
        PipelineIter {
            tokens: self.tokens.clone(),
            all_tokens: self.tokens.clone(),
            pipeline: self,
            i: 0,
            last: false,
        }
    }
}

impl<'a> PipelineIter<'a> {
    fn inner_next(&mut self) -> Result<Generation, Error> {
        // tracing::info!(
        //     "Inner next {:?} - {:?}",
        //     self.tokens,
        //     self.pipeline
        //         .tokenizer
        //         .decode(self.tokens.as_slice(), false)
        // );
        let input = Tensor::new(self.tokens.as_slice(), &Device::Cpu)?.unsqueeze(0)?;
        let logits = self.pipeline.model.forward(&input, 0)?;

        // Once for batch size
        let logits = logits.squeeze(0)?;
        // Once for seq len, logits processor goes crazy otherwise.
        let logits = logits.squeeze(0)?;

        let next_token = self.pipeline.logits_processor.sample(&logits)?;
        self.all_tokens.push(next_token);
        let text = print_token(next_token, &self.pipeline.tokenizer);
        self.all_tokens.push(next_token);

        self.tokens = vec![next_token];
        let generated_text = if self.i == self.pipeline.query.parameters.max_new_tokens {
            Some(self.pipeline.tokenizer.decode(&self.all_tokens, true)?)
        } else {
            None
        };
        self.i += 1;
        let generation = Generation {
            token: Token {
                id: next_token as usize,
                logprob: 0.0,
                text,
                special: false,
            },
            generated_text,
            details: None,
        };
        Ok(generation)
    }
}
impl<'a> Iterator for PipelineIter<'a> {
    type Item = Result<Generation, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last {
            None
        } else {
            let generation = self.inner_next();
            if let Ok(generation) = &generation {
                if generation.generated_text.is_some() {
                    self.last = true;
                }
            }
            Some(generation)
        }
    }
}