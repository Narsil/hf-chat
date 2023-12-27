#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use tokenizers::Tokenizer;

use crate::{Error, Generation, Query, Token};
use candle::quantized::{ggml_file, gguf_file};
use candle::{Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Cache};

use candle_transformers::models::quantized_llama as model;
use model::ModelWeights;
use tracing::info;

fn tokenizer(api: &Api) -> Result<Tokenizer, Error> {
    let api = api.model("hf-internal-testing/llama-tokenizer".to_string());
    let tokenizer_path = api.get("tokenizer.json")?;
    Ok(Tokenizer::from_file(tokenizer_path)?)
}

fn get_model(api: &Api) -> Result<ModelWeights, Error> {
    let (repo, filename) = (
        "TheBloke/Llama-2-7B-Chat-GGML",
        "llama-2-7b-chat.ggmlv3.q4_0.bin",
    );
    let api = api.model(repo.to_string());
    info!("Getting {filename}");
    let model_path = api.get(filename)?;
    info!("Got {filename}");
    let start = std::time::Instant::now();
    let mut file = std::fs::File::open(&model_path)?;
    let model: ModelWeights = match model_path.extension().and_then(|v| v.to_str()) {
        Some("gguf") => {
            let model = gguf_file::Content::read(&mut file)?;
            let mut total_size_in_bytes = 0;
            for (_, tensor) in model.tensor_infos.iter() {
                let elem_count = tensor.shape.elem_count();
                total_size_in_bytes +=
                    elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.blck_size();
            }
            info!(
                "loaded {:?} tensors ({}) in {:.2}s",
                model.tensor_infos.len(),
                &format_size(total_size_in_bytes),
                start.elapsed().as_secs_f32(),
            );
            ModelWeights::from_gguf(model, &mut file)?
        }
        Some("ggml" | "bin") | Some(_) | None => {
            let model = ggml_file::Content::read(&mut file)?;
            let mut total_size_in_bytes = 0;
            for (_, tensor) in model.tensors.iter() {
                let elem_count = tensor.shape().elem_count();
                total_size_in_bytes +=
                    elem_count * tensor.dtype().type_size() / tensor.dtype().blck_size();
            }
            info!(
                "loaded {:?} tensors ({}) in {:.2}s",
                model.tensors.len(),
                &format_size(total_size_in_bytes),
                start.elapsed().as_secs_f32(),
            );
            info!("params: {:?}", model.hparams);
            let default_gqa = 1;
            ModelWeights::from_ggml(model, default_gqa)?
        }
    };
    Ok(model)
}

fn print_token(next_token: u32, tokenizer: &Tokenizer) -> String {
    // Extracting the last token as a string is complicated, here we just apply some simple
    // heuristics as it seems to work well enough for this example. See the following for more
    // details:
    // https://github.com/huggingface/tokenizers/issues/1141#issuecomment-1562644141
    if let Some(text) = tokenizer.id_to_token(next_token) {
        let text = text.replace('▁', " ");
        let ascii = text
            .strip_prefix("<0x")
            .and_then(|t| t.strip_suffix('>'))
            .and_then(|t| u8::from_str_radix(t, 16).ok());
        match ascii {
            None => return text,
            Some(ascii) => {
                if let Some(chr) = char::from_u32(ascii as u32) {
                    if chr.is_ascii() {
                        return format!("{chr}");
                    }
                }
            }
        }
    }
    "".into()
}

fn format_size(size_in_bytes: usize) -> String {
    if size_in_bytes < 1_000 {
        format!("{}B", size_in_bytes)
    } else if size_in_bytes < 1_000_000 {
        format!("{:.2}KB", size_in_bytes as f64 / 1e3)
    } else if size_in_bytes < 1_000_000_000 {
        format!("{:.2}MB", size_in_bytes as f64 / 1e6)
    } else {
        format!("{:.2}GB", size_in_bytes as f64 / 1e9)
    }
}

pub struct Pipeline {
    model: ModelWeights,
    tokenizer: Tokenizer,
    device: Device,
    query: Query,
    tokens: Vec<u32>,
    logits_processor: LogitsProcessor,
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
        device,
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
            all_tokens: vec![],
            pipeline: self,
            i: 0,
            last: false,
        }
    }
}

impl<'a> PipelineIter<'a> {
    fn inner_next(&mut self) -> Result<Generation, Error> {
        tracing::info!(
            "Inner next {:?} - {:?}",
            self.tokens,
            self.pipeline
                .tokenizer
                .decode(self.tokens.as_slice(), false)
        );
        let input = Tensor::new(self.tokens.as_slice(), &self.pipeline.device)?.unsqueeze(0)?;
        tracing::debug!("input {:?}", input.shape());
        let logits = self.pipeline.model.forward(&input, 0)?;
        tracing::debug!("Logits {:?}", logits.shape());
        let logits = logits.squeeze(0)?;
        let next_token = self.pipeline.logits_processor.sample(&logits)?;
        self.all_tokens.push(next_token);
        let text = print_token(next_token, &self.pipeline.tokenizer);

        self.tokens = vec![next_token];
        let parameters = &self.pipeline.query.parameters;
        let generated_text = if self.i == parameters.max_new_tokens
            || parameters.stop.iter().any(|stop| text.starts_with(stop))
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
