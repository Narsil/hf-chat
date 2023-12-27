#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use crate::{Error, Generation, Query, Token};
use candle_transformers::models::mixformer::Config;
use candle_transformers::models::quantized_mixformer::MixFormerSequentialForCausalLM as QMixFormer;

use candle::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Cache};
use hf_hub::{Repo, RepoType};
use tokenizers::Tokenizer;
use tracing::info;

fn tokenizer(api: &Api) -> Result<Tokenizer, Error> {
    let model_id = "microsoft/phi-1_5".to_string();
    let revision = "refs/pr/18".to_string();
    let repo = api.repo(Repo::with_revision(model_id, RepoType::Model, revision));
    let tokenizer_filename = repo.get("tokenizer.json")?;
    Ok(Tokenizer::from_file(tokenizer_filename)?)
}

fn get_model(api: &Api) -> Result<QMixFormer, Error> {
    let model_id = "lmz/candle-quantized-phi".to_string();
    let repo = api.repo(Repo::new(model_id, RepoType::Model));
    info!("Getting phi model");
    let filename = repo.get("model-q4k.gguf")?;
    info!("Got phi model");
    let config = Config::v1_5();
    let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(&filename)?;
    let model = QMixFormer::new(&config, vb)?;
    Ok(model)
}

pub fn load_local(query: Query, device: Device, cache: &Cache) -> Result<Pipeline, Error> {
    let api = hf_hub::api::sync::ApiBuilder::from_cache(cache.clone()).build()?;
    let tokenizer = tokenizer(&api)?;
    let model = get_model(&api)?;
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
        device,
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
    count: usize,
}

pub struct Pipeline {
    model: QMixFormer,
    tokenizer: Tokenizer,
    device: Device,
    query: Query,
    tokens: Vec<u32>,
    logits_processor: LogitsProcessor,
}
impl Pipeline {
    pub fn iter(&mut self) -> PipelineIter {
        PipelineIter {
            tokens: self.tokens.clone(),
            all_tokens: vec![],
            pipeline: self,
            i: 0,
            count: 0,
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
        let input = Tensor::new(self.tokens.as_slice(), &self.pipeline.device)?.unsqueeze(0)?;
        let logits = self.pipeline.model.forward(&input)?;

        // Once for batch size
        let logits = logits.squeeze(0)?;
        // Once for seq len, logits processor goes crazy otherwise.
        let logits = logits.squeeze(0)?;

        let next_token = self.pipeline.logits_processor.sample(&logits)?;
        self.all_tokens.push(next_token);
        let text = print_token(next_token, &self.pipeline.tokenizer);
        if text == "\n" {
            self.count += 1;
        } else {
            self.count = 1;
        }

        let mut stop = false;
        if self.count == 3 {
            // 3  means 1 for having actual text, and + 2 for newlines
            stop = true;
        }
        let parameters = &self.pipeline.query.parameters;
        if self.i == parameters.max_new_tokens {
            stop = true;
        }

        self.tokens = vec![next_token];
        let generated_text = if stop
        // || parameters.stop.iter().any(|stop| text.starts_with(stop))
        {
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

fn print_token(next_token: u32, tokenizer: &Tokenizer) -> String {
    tokenizer.decode(&[next_token], true).unwrap()
}
