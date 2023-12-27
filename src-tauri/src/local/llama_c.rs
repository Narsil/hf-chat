use byteorder::{LittleEndian, ReadBytesExt};
use candle::{DType, Device, IndexOp, Shape, Tensor, D};

// https://github.com/karpathy/llama2.c

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

use crate::{Error, Generation, Query, Token};
use candle_nn::linear_no_bias as linear;
use candle_nn::{embedding, rms_norm, Embedding, Linear, Module, RmsNorm, VarBuilder};
use candle_transformers::generation::LogitsProcessor;
use hf_hub::{api::sync::Api, Cache as HfCache};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;
use tracing::info;

// #[derive(Parser, Debug)]
// #[command(author, version, about, long_about = None)]
// pub struct Args {
//     /// The task to be performed, inference, training or evaluation.
//     #[command(subcommand)]
//     task: Option<Task>,
//
//     /// Run on CPU rather than on GPU.
//     #[arg(long)]
//     cpu: bool,
//
//     /// Tokenizer config file.
//     #[arg(long)]
//     tokenizer: Option<String>,
//
//     /// Penalty to be applied for repeating tokens, 1. means no penalty.
//     #[arg(long, default_value_t = 1.1)]
//     repeat_penalty: f32,
//
//     /// The context size to consider for the repeat penalty.
//     #[arg(long, default_value_t = 64)]
//     repeat_last_n: usize,
// }

fn tokenizer(api: &Api) -> Result<Tokenizer, Error> {
    let api = api.model("hf-internal-testing/llama-tokenizer".to_string());
    let tokenizer_path = api.get("tokenizer.json")?;
    Ok(Tokenizer::from_file(tokenizer_path)?)
}

fn get_model(api: &Api, device: &Device) -> Result<Llama, Error> {
    let (repo, filename) = ("karpathy/tinyllamas", "stories15M.bin");
    info!("loading the model weights from {}", repo);
    let api = api.model(repo.into());
    let model_path = api.get(filename)?;

    let is_safetensors = model_path.extension().map_or(false, |v| v == "safetensors");
    let (vb, config) = if is_safetensors {
        let config = Config::tiny();
        let tensors = candle::safetensors::load(model_path, &device)?;
        let vb = candle_nn::VarBuilder::from_tensors(tensors, candle::DType::F32, &device);
        (vb, config)
    } else {
        let mut file = std::fs::File::open(model_path)?;
        let config = Config::from_reader(&mut file)?;
        info!("{config:?}");
        let weights = TransformerWeights::from_reader(&mut file, &config, &device)?;
        let vb = weights.var_builder(&config, &device)?;
        (vb, config)
    };
    let cache = Cache::new(true, &config, vb.pp("rot"))?;
    let model = Llama::load(vb, &cache, config)?;
    Ok(model)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub dim: usize,        // transformer dimension
    pub hidden_dim: usize, // for ffn layers
    pub n_layers: usize,   // number of layers
    pub n_heads: usize,    // number of query heads
    pub n_kv_heads: usize, // number of key/value heads (can be < query heads because of multiquery)
    pub vocab_size: usize, // vocabulary size, usually 256 (byte-level)
    pub seq_len: usize,    // max sequence length
    pub norm_eps: f64,
}

impl Config {
    pub fn tiny() -> Self {
        Self {
            dim: 288,
            hidden_dim: 768,
            n_layers: 6,
            n_heads: 6,
            n_kv_heads: 6,
            vocab_size: 32000,
            seq_len: 256,
            norm_eps: 1e-5,
        }
    }
}

#[derive(Clone)]
pub struct Cache {
    masks: Arc<Mutex<HashMap<usize, Tensor>>>,
    pub use_kv_cache: bool,
    #[allow(clippy::type_complexity)]
    kvs: Arc<Mutex<Vec<Option<(Tensor, Tensor)>>>>,
    cos: Tensor,
    sin: Tensor,
    device: Device,
}

impl Cache {
    pub fn new(use_kv_cache: bool, cfg: &Config, vb: VarBuilder) -> Result<Self, Error> {
        let n_elem = cfg.dim / cfg.n_heads;
        let theta: Vec<_> = (0..n_elem)
            .step_by(2)
            .map(|i| 1f32 / 10000f32.powf(i as f32 / n_elem as f32))
            .collect();
        let theta = Tensor::new(theta.as_slice(), vb.device())?;
        let idx_theta = Tensor::arange(0, cfg.seq_len as u32, vb.device())?
            .to_dtype(DType::F32)?
            .reshape((cfg.seq_len, 1))?
            .matmul(&theta.reshape((1, theta.elem_count()))?)?;
        let precomputed_cos = idx_theta.cos()?;
        let precomputed_sin = idx_theta.sin()?;

        let freq_cis_real = vb
            .get((cfg.seq_len, cfg.head_size() / 2), "freq_cis_real")
            .unwrap_or(precomputed_cos);
        let freq_cis_imag = vb
            .get((cfg.seq_len, cfg.head_size() / 2), "freq_cis_imag")
            .unwrap_or(precomputed_sin);
        let cos = freq_cis_real.reshape((cfg.seq_len, cfg.head_size() / 2, 1))?;
        let sin = freq_cis_imag.reshape((cfg.seq_len, cfg.head_size() / 2, 1))?;
        Ok(Self {
            masks: Arc::new(Mutex::new(HashMap::new())),
            use_kv_cache,
            kvs: Arc::new(Mutex::new(vec![None; cfg.n_layers])),
            cos,
            sin,
            device: vb.device().clone(),
        })
    }

    fn mask(&self, t: usize) -> Result<Tensor, Error> {
        let mut masks = self.masks.lock().unwrap();
        if let Some(mask) = masks.get(&t) {
            Ok(mask.clone())
        } else {
            let mask: Vec<_> = (0..t)
                .flat_map(|i| (0..t).map(move |j| u8::from(j > i)))
                .collect();
            let mask = Tensor::from_slice(&mask, (t, t), &self.device)?;
            masks.insert(t, mask.clone());
            Ok(mask)
        }
    }
}

fn silu(xs: &Tensor) -> Result<Tensor, Error> {
    Ok((xs / (xs.neg()?.exp()? + 1.0)?)?)
}

struct CausalSelfAttention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    n_head: usize,
    n_key_value_head: usize,
    head_dim: usize,
    cache: Cache,
}

impl CausalSelfAttention {
    fn apply_rotary_emb(&self, x: &Tensor, index_pos: usize) -> Result<Tensor, Error> {
        let (b_sz, seq_len, h, n_embd) = x.dims4()?;
        let cos = self.cache.cos.i(index_pos..index_pos + seq_len)?;
        let sin = self.cache.sin.i(index_pos..index_pos + seq_len)?;
        let cos = cos.unsqueeze(1)?;
        let sin = sin.unsqueeze(1)?;
        let cos = cos.broadcast_as((b_sz, seq_len, 1, n_embd / 2, 1))?;
        let sin = sin.broadcast_as((b_sz, seq_len, 1, n_embd / 2, 1))?;
        let x = x.reshape((b_sz, seq_len, h, n_embd / 2, 2))?;
        let x0 = x.narrow(D::Minus1, 0, 1)?;
        let x1 = x.narrow(D::Minus1, 1, 1)?;
        let dst0 = (x0.broadcast_mul(&cos)? - x1.broadcast_mul(&sin)?)?;
        let dst1 = (x0.broadcast_mul(&sin)? + x1.broadcast_mul(&cos)?)?;
        let rope = Tensor::cat(&[&dst0, &dst1], D::Minus1)?.reshape((b_sz, seq_len, h, n_embd))?;
        Ok(rope)
    }

    fn forward(&self, x: &Tensor, index_pos: usize, block_idx: usize) -> Result<Tensor, Error> {
        let (b_sz, seq_len, n_embd) = x.dims3()?;
        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        let q = q.reshape((b_sz, seq_len, self.n_head, self.head_dim))?;
        let k = k.reshape((b_sz, seq_len, self.n_key_value_head, self.head_dim))?;
        let mut v = v.reshape((b_sz, seq_len, self.n_key_value_head, self.head_dim))?;

        let q = self.apply_rotary_emb(&q, index_pos)?;
        let mut k = self.apply_rotary_emb(&k, index_pos)?;

        if self.cache.use_kv_cache {
            let mut cache = self.cache.kvs.lock().unwrap();
            if let Some((cache_k, cache_v)) = &cache[block_idx] {
                k = Tensor::cat(&[cache_k, &k], 1)?.contiguous()?;
                v = Tensor::cat(&[cache_v, &v], 1)?.contiguous()?;
            }
            cache[block_idx] = Some((k.clone(), v.clone()))
        }

        let k = self.repeat_kv(k)?;
        let v = self.repeat_kv(v)?;

        let q = q.transpose(1, 2)?.contiguous()?;
        let k = k.transpose(1, 2)?.contiguous()?;
        let v = v.transpose(1, 2)?.contiguous()?;

        let att = (q.matmul(&k.t()?)? / (self.head_dim as f64).sqrt())?;
        let mask = self.cache.mask(seq_len)?.broadcast_as(att.shape())?;
        let att = masked_fill(&att, &mask, f32::NEG_INFINITY)?;
        let att = candle_nn::ops::softmax(&att, D::Minus1)?;
        // Convert to contiguous as matmul doesn't support strided vs for now.
        let y = att.matmul(&v.contiguous()?)?;
        let y = y.transpose(1, 2)?.reshape(&[b_sz, seq_len, n_embd])?;
        let y = self.o_proj.forward(&y)?;
        Ok(y)
    }

    fn repeat_kv(&self, x: Tensor) -> Result<Tensor, Error> {
        let n_rep = self.n_head / self.n_key_value_head;
        if n_rep == 1 {
            Ok(x)
        } else {
            let (b_sz, seq_len, n_kv_head, head_dim) = x.dims4()?;
            let x = x
                .unsqueeze(3)?
                .expand((b_sz, seq_len, n_kv_head, n_rep, head_dim))?
                .reshape((b_sz, seq_len, n_kv_head * n_rep, head_dim))?;
            Ok(x)
        }
    }

    fn load(vb: VarBuilder, cache: &Cache, cfg: &Config) -> Result<Self, Error> {
        let size_in = cfg.dim;
        let size_q = (cfg.dim / cfg.n_heads) * cfg.n_heads;
        let size_kv = (cfg.dim / cfg.n_heads) * cfg.n_kv_heads;
        let q_proj = linear(size_in, size_q, vb.pp("q_proj"))?;
        let k_proj = linear(size_in, size_kv, vb.pp("k_proj"))?;
        let v_proj = linear(size_in, size_kv, vb.pp("v_proj"))?;
        let o_proj = linear(size_q, size_in, vb.pp("o_proj"))?;
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            n_head: cfg.n_heads,
            n_key_value_head: cfg.n_kv_heads,
            head_dim: cfg.dim / cfg.n_heads,
            cache: cache.clone(),
        })
    }
}

fn masked_fill(on_false: &Tensor, mask: &Tensor, on_true: f32) -> Result<Tensor, Error> {
    let shape = mask.shape();
    let on_true = Tensor::new(on_true, on_false.device())?.broadcast_as(shape.dims())?;
    let m = mask.where_cond(&on_true, on_false)?;
    Ok(m)
}

struct Mlp {
    c_fc1: Linear,
    c_fc2: Linear,
    c_proj: Linear,
}

impl Mlp {
    fn new(c_fc1: Linear, c_fc2: Linear, c_proj: Linear) -> Self {
        Self {
            c_fc1,
            c_fc2,
            c_proj,
        }
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor, Error> {
        let x: Tensor = (silu(&self.c_fc1.forward(x)?)? * self.c_fc2.forward(x)?)?;
        Ok(self.c_proj.forward(&x)?)
    }

    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self, Error> {
        let h_size = cfg.dim;
        let i_size = cfg.hidden_dim;
        let c_fc1 = linear(h_size, i_size, vb.pp("gate_proj"))?;
        let c_fc2 = linear(h_size, i_size, vb.pp("up_proj"))?;
        let c_proj = linear(i_size, h_size, vb.pp("down_proj"))?;
        Ok(Self::new(c_fc1, c_fc2, c_proj))
    }
}

struct Block {
    rms_1: RmsNorm,
    attn: CausalSelfAttention,
    rms_2: RmsNorm,
    mlp: Mlp,
}

impl Block {
    fn new(rms_1: RmsNorm, attn: CausalSelfAttention, rms_2: RmsNorm, mlp: Mlp) -> Self {
        Self {
            rms_1,
            attn,
            rms_2,
            mlp,
        }
    }

    fn forward(&self, x: &Tensor, index_pos: usize, block_idx: usize) -> Result<Tensor, Error> {
        let residual = x;
        let x = self.rms_1.forward(x)?;
        let x = (self.attn.forward(&x, index_pos, block_idx)? + residual)?;
        let residual = &x;
        let x = (self.mlp.forward(&self.rms_2.forward(&x)?)? + residual)?;
        Ok(x)
    }

    fn load(vb: VarBuilder, cache: &Cache, cfg: &Config) -> Result<Self, Error> {
        let attn = CausalSelfAttention::load(vb.pp("self_attn"), cache, cfg)?;
        let mlp = Mlp::load(vb.pp("mlp"), cfg)?;
        let input_layernorm = rms_norm(cfg.dim, cfg.norm_eps, vb.pp("input_layernorm"))?;
        let post_attention_layernorm =
            rms_norm(cfg.dim, cfg.norm_eps, vb.pp("post_attention_layernorm"))?;
        Ok(Self::new(
            input_layernorm,
            attn,
            post_attention_layernorm,
            mlp,
        ))
    }
}

pub struct Llama {
    wte: Embedding,
    blocks: Vec<Block>,
    ln_f: RmsNorm,
    lm_head: Linear,
    pub config: Config,
}

impl Llama {
    pub fn forward(&self, x: &Tensor, index_pos: usize) -> Result<Tensor, Error> {
        let (_b_sz, seq_len) = x.dims2()?;
        let mut x = self.wte.forward(x)?;
        for (block_idx, block) in self.blocks.iter().enumerate() {
            x = block.forward(&x, index_pos, block_idx)?;
        }
        let x = self.ln_f.forward(&x)?;

        let x = x.i((.., seq_len - 1..))?;
        let logits = self.lm_head.forward(&x)?;
        Ok(logits.to_dtype(DType::F32)?)
    }

    pub fn load(vb: VarBuilder, cache: &Cache, cfg: Config) -> Result<Self, Error> {
        let wte = embedding(cfg.vocab_size, cfg.dim, vb.pp("model.embed_tokens"))?;
        let lm_head = linear(cfg.dim, cfg.vocab_size, vb.pp("lm_head"))?;
        let ln_f = rms_norm(cfg.dim, cfg.norm_eps, vb.pp("model.norm"))?;
        let blocks: Vec<_> = (0..cfg.n_layers)
            .map(|i| Block::load(vb.pp(&format!("model.layers.{i}")), cache, &cfg).unwrap())
            .collect();
        Ok(Self {
            wte,
            blocks,
            ln_f,
            lm_head,
            config: cfg,
        })
    }
}

pub struct TransformerWeights {
    // token embedding table
    token_embedding_table: Tensor, // (vocab_size, dim)
    // weights for rmsnorms
    rms_att_weight: Tensor, // (layer, dim) rmsnorm weights
    rms_ffn_weight: Tensor, // (layer, dim)
    // weights for matmuls
    wq: Tensor, // (layer, dim, dim)
    wk: Tensor, // (layer, dim, dim)
    wv: Tensor, // (layer, dim, dim)
    wo: Tensor, // (layer, dim, dim)
    // weights for ffn
    w1: Tensor, // (layer, hidden_dim, dim)
    w2: Tensor, // (layer, dim, hidden_dim)
    w3: Tensor, // (layer, hidden_dim, dim)
    // final rmsnorm
    rms_final_weight: Tensor, // (dim,)
    // freq_cis for RoPE relatively positional embeddings
    freq_cis_real: Tensor, // (seq_len, head_size/2)
    freq_cis_imag: Tensor, // (seq_len, head_size/2)
}

fn read_i32<R: std::io::Read>(r: &mut R) -> Result<i32, Error> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_tensor<R: std::io::Read, S: Into<Shape>>(
    r: &mut R,
    shape: S,
    dev: &Device,
) -> Result<Tensor, Error> {
    let shape = shape.into();
    let mut data_t = vec![0f32; shape.elem_count()];
    r.read_f32_into::<LittleEndian>(&mut data_t)?;
    let tensor = Tensor::from_vec(data_t, shape, dev)?;
    Ok(tensor)
}

impl Config {
    pub fn from_reader<R: std::io::Read>(r: &mut R) -> Result<Self, Error> {
        let dim = read_i32(r)? as usize;
        let hidden_dim = read_i32(r)? as usize;
        let n_layers = read_i32(r)? as usize;
        let n_heads = read_i32(r)? as usize;
        let n_kv_heads = read_i32(r)? as usize;
        let vocab_size = read_i32(r)? as usize;
        let seq_len = read_i32(r)? as usize;
        Ok(Self {
            dim,
            hidden_dim,
            n_layers,
            n_heads,
            n_kv_heads,
            vocab_size,
            seq_len,
            norm_eps: 1e-5,
        })
    }

    pub fn head_size(&self) -> usize {
        self.dim / self.n_heads
    }
}

impl TransformerWeights {
    pub fn from_reader<R: std::io::Read>(
        r: &mut R,
        c: &Config,
        dev: &Device,
    ) -> Result<Self, Error> {
        let token_embedding_table = read_tensor(r, (c.vocab_size, c.dim), dev)?;
        let rms_att_weight = read_tensor(r, (c.n_layers, c.dim), dev)?;
        let wq = read_tensor(r, (c.n_layers, c.dim, c.dim), dev)?;
        let wk = read_tensor(r, (c.n_layers, c.dim, c.dim), dev)?;
        let wv = read_tensor(r, (c.n_layers, c.dim, c.dim), dev)?;
        let wo = read_tensor(r, (c.n_layers, c.dim, c.dim), dev)?;
        let rms_ffn_weight = read_tensor(r, (c.n_layers, c.dim), dev)?;
        let w1 = read_tensor(r, (c.n_layers, c.hidden_dim, c.dim), dev)?;
        let w2 = read_tensor(r, (c.n_layers, c.dim, c.hidden_dim), dev)?;
        let w3 = read_tensor(r, (c.n_layers, c.hidden_dim, c.dim), dev)?;
        let rms_final_weight = read_tensor(r, c.dim, dev)?;
        let head_size = c.head_size();
        let freq_cis_real = read_tensor(r, (c.seq_len, head_size / 2), dev)?;
        let freq_cis_imag = read_tensor(r, (c.seq_len, head_size / 2), dev)?;
        Ok(Self {
            token_embedding_table,
            rms_att_weight,
            wq,
            wk,
            wv,
            wo,
            rms_ffn_weight,
            w1,
            w2,
            w3,
            rms_final_weight,
            freq_cis_real,
            freq_cis_imag,
        })
    }

    pub fn var_builder(&self, cfg: &Config, device: &Device) -> Result<VarBuilder<'static>, Error> {
        // TODO: As of 2023-08-04, gemm is slower than expected when multiplying a matrix of
        // size (1, k) with the transpose of a matrix of size (k, n) as it ends up transposing the
        // second matrix back. We detect this case here and as a temporary hack make the weight
        // matrix column major rather than row major. This ends up speeding up text generation from
        // 120 token/s to 220 token/s on a Ryzen 2600X.
        let tr = device.is_cpu() && !candle::utils::has_mkl();
        let tr = |x: Tensor| if tr { x.t()?.contiguous()?.t() } else { Ok(x) };
        let mut ws = std::collections::HashMap::new();
        let mut insert = |name: &str, t: Tensor| {
            ws.insert(name.to_string(), t);
        };
        insert("rot.freq_cis_real", self.freq_cis_real.clone());
        insert("rot.freq_cis_imag", self.freq_cis_imag.clone());
        insert(
            "model.embed_tokens.weight",
            self.token_embedding_table.clone(),
        );
        insert("lm_head.weight", tr(self.token_embedding_table.clone())?);
        insert("model.norm.weight", self.rms_final_weight.clone());
        for layer in 0..cfg.n_layers {
            ws.insert(
                format!("model.layers.{layer}.self_attn.q_proj.weight"),
                tr(self.wq.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.self_attn.k_proj.weight"),
                tr(self.wk.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.self_attn.v_proj.weight"),
                tr(self.wv.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.self_attn.o_proj.weight"),
                tr(self.wo.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.mlp.gate_proj.weight"),
                tr(self.w1.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.mlp.down_proj.weight"),
                tr(self.w2.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.mlp.up_proj.weight"),
                tr(self.w3.i(layer)?)?,
            );
            ws.insert(
                format!("model.layers.{layer}.input_layernorm.weight"),
                self.rms_att_weight.i(layer)?,
            );
            ws.insert(
                format!("model.layers.{layer}.post_attention_layernorm.weight"),
                self.rms_ffn_weight.i(layer)?,
            );
        }
        let vb = VarBuilder::from_tensors(ws, DType::F32, device);
        Ok(vb)
    }
}

pub struct Pipeline {
    model: Llama,
    tokenizer: Tokenizer,
    device: Device,
    query: Query,
    tokens: Vec<u32>,
    logits_processor: LogitsProcessor,
}
// info!("starting the inference loop");
// let mut logits_processor = LogitsProcessor::new(299792458, args.temperature, args.top_p);
// let mut index_pos = 0;

// let mut tokens = tokenizer
//     .encode(query.inputs, true)?
//     .get_ids()
//     .to_vec();

// let start_gen = std::time::Instant::now();
// for index in 0.. {
//     if tokens.len() >= model.config.seq_len {
//         break;
//     }
//     let context_size = if index > 0 { 1 } else { tokens.len() };
//     let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];
//     let input = Tensor::new(ctxt, &device)?.unsqueeze(0)?;
//     let logits = model.forward(&input, index_pos)?;
//     let logits = logits.i((0, logits.dim(1)? - 1))?;
//     let logits = if common_args.repeat_penalty == 1. || tokens.is_empty() {
//         logits
//     } else {
//         let start_at = tokens.len().saturating_sub(common_args.repeat_last_n);
//         candle_transformers::utils::apply_repeat_penalty(
//             &logits,
//             common_args.repeat_penalty,
//             &tokens[start_at..],
//         )?
//     };
//     index_pos += ctxt.len();

//     let next_token = logits_processor.sample(&logits)?;
//     tokens.push(next_token);
//     // Extracting the last token as a string is complicated, here we just apply some simple
//     // heuristics as it seems to work well enough for this example. See the following for more
//     // details:
//     // https://github.com/huggingface/tokenizers/issues/1141#issuecomment-1562644141
//     if let Some(text) = tokenizer.id_to_token(next_token) {
//         let text = text.replace('▁', " ").replace("<0x0A>", "\n");
//         print!("{text}");
//         std::io::stdout().flush()?;
//     }
// }
// let dt = start_gen.elapsed();
// info!(
//     "\n{} tokens generated ({:.2} token/s)\n",
//     tokens.len(),
//     tokens.len() as f64 / dt.as_secs_f64(),
// );
// Ok(())

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

pub fn load_local(query: Query, device: Device, cache: &HfCache) -> Result<Pipeline, Error> {
    let api = hf_hub::api::sync::ApiBuilder::from_cache(cache.clone()).build()?;
    let tokenizer = tokenizer(&api)?;
    let model = get_model(&api, &device)?;
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
        // tracing::info!(
        //     "Inner next {:?} - {:?}",
        //     self.tokens,
        //     self.pipeline
        //         .tokenizer
        //         .decode(self.tokens.as_slice(), false)
        // );
        let input = Tensor::new(self.tokens.as_slice(), &self.pipeline.device)?.unsqueeze(0)?;
        let logits = self.pipeline.model.forward(&input, 0)?;

        // Once for batch size
        let logits = logits.squeeze(0)?;
        // Once for seq len, logits processor goes crazy otherwise.
        let logits = logits.squeeze(0)?;

        let next_token = self.pipeline.logits_processor.sample(&logits)?;
        self.all_tokens.push(next_token);
        let text = print_token(next_token, &self.pipeline.tokenizer);

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
