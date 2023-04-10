use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::option::Option;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::vec::Vec;

/// A struct representing the decoding arguments for OpenAI API requests
#[derive(Debug, PartialEq, Clone)]
pub struct OpenAIDecodingArguments {
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub n: usize,
    pub stream: bool,
    pub stop: Option<Vec<String>>,
    pub presence_penalty: f32,
    pub frequency_penalty: f32,
    pub suffix: Option<String>,
    pub logprobs: Option<usize>,
    pub echo: bool,
}

/// Implement the Default trait for OpenAIDecodingArguments
impl Default for OpenAIDecodingArguments {
    // Provide a default set of values for the decoding arguments
    fn default() -> Self {
        OpenAIDecodingArguments {
            max_tokens: 1800,
            temperature: 0.2,
            top_p: 1.0,
            n: 1,
            stream: false,
            stop: None,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            suffix: None,
            logprobs: None,
            echo: false,
        }
    }
}

/// Determines if the provided prompt is a single prompt or not.
///
/// A single prompt can be represented by either a string or an object in the given data format.
/// This function takes a reference to a serde_json::Value and returns a boolean value.
///
/// - If the prompt is a string, it means it's a single prompt text.
/// - If the prompt is an object, it means it's a single prompt formatted as a dictionary (e.g., for chat models).
///
/// Returns true if the prompt is either a string or an object, indicating that it's a single prompt,
/// and false otherwise.
fn is_single_prompt(prompt: &Value) -> bool {
    prompt.is_string() || prompt.is_object()
}

/// Prepares the prompt batches for processing by splitting the input prompts into smaller chunks.
///
/// This function takes a Vec<Value> of prompts, a batch_size, and a max_instances limit.
/// It trims the prompts to max_instances and then splits them into smaller batches of the specified size.
///
/// # Arguments
///
/// * prompts - A vector of prompts represented as serde_json::Value.
/// * batch_size - The maximum number of prompts in a single batch.
/// * max_instances - The maximum number of prompts to process.
///
/// Returns a vector of vectors, each containing a batch of prompts.
fn prepare_prompt_batches(
    prompts: Vec<Value>,
    batch_size: usize,
    max_instances: usize,
) -> Vec<Vec<Value>> {
    let prompts = prompts.into_iter().take(max_instances).collect::<Vec<_>>();
    let num_prompts = prompts.len();
    let num_prompt_batches = (num_prompts as f64 / batch_size as f64).ceil() as usize;

    (0..num_prompt_batches)
        .map(|batch_id| {
            let start = batch_id * batch_size;
            let end = (batch_id + 1) * batch_size;
            prompts[start..end].to_vec()
        })
        .collect()
}

/// Asynchronously sends a request to the OpenAI API with the specified parameters and model.
///
/// This function takes an HTTP Client, the API url, an api_key, a prompt_batch,
/// decoding_args, and decoding_kwargs. It sends a request to the OpenAI API using
/// the provided model and returns the completion choices as a Result<Vec<Value>>, /// Box<dyn Error>>.
///
/// # Arguments
/// * client - An HTTP client instance for sending the request.
/// * url - The OpenAI API endpoint URL.
/// * api_key - The API key for authentication with OpenAI.
/// * prompt_batch - A slice of prompts to send to the API.
/// * decoding_args - Decoding arguments for the API.
/// * decoding_kwargs - Additional decoding arguments as a HashMap.
///
/// # Returns
/// * A Result containing a Vec<Value> of completion choices, or a Box<dyn Error>
/// if an error occurs.
async fn send_request(
    client: &Client,
    url: &str,
    api_key: &str,
    prompt_batch: &[Value],
    decoding_args: &OpenAIDecodingArguments,
    decoding_kwargs: &HashMap<String, Value>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut request_data = HashMap::new();
    request_data.insert("model".to_string(), "text-davinci-003".to_string());
    request_data.insert("prompt".to_string(), serde_json::to_string(prompt_batch)?);
    request_data.insert(
        "max_tokens".to_string(),
        decoding_args.max_tokens.to_string(),
    );
    request_data.insert(
        "temperature".to_string(),
        decoding_args.temperature.to_string(),
    );
    request_data.insert("top_p".to_string(), decoding_args.top_p.to_string());
    request_data.insert("n".to_string(), decoding_args.n.to_string());
    request_data.insert("stream".to_string(), decoding_args.stream.to_string());
    request_data.insert(
        "presence_penalty".to_string(),
        decoding_args.presence_penalty.to_string(),
    );
    request_data.insert(
        "frequency_penalty".to_string(),
        decoding_args.frequency_penalty.to_string(),
    );
    request_data.insert("echo".to_string(), decoding_args.echo.to_string());

    for (key, value) in decoding_kwargs {
        let value_str = value.to_string();
        request_data.insert(key.clone(), value_str);
    }

    let response = client
        .post(url)
        .json(&request_data)
        .bearer_auth(api_key)
        .send()
        .await?;

    if response.status() != StatusCode::OK {
        return Err(format!("OpenAIError: {}", response.status()).into());
    }
    let completion_batch: Value = response.json().await?;
    let choices = completion_batch["choices"].as_array().unwrap();
    Ok(choices.iter().cloned().collect())
}

/// Sends a request to the OpenAI API to generate completions for the given prompt(s).
///
/// # Arguments
/// * `prompt` - A single `Value` or an array of `Value`s representing the input prompt(s).
/// * `decoding_args` - An `OpenAIDecodingArguments` struct containing decoding options for the API request.
/// * `model_name` - A string slice with the name of the OpenAI model to use (e.g. "text-davinci-003").
/// * `sleep_time` - The number of seconds to sleep between retries when the rate limit is hit.
/// * `batch_size` - The number of prompts to send in each request batch.
/// * `max_instances` - The maximum number of instances (prompts) to process.
/// * `return_text` - If `true`, only the generated text will be returned in the response; if `false`, the entire response object will be returned.
/// * `decoding_kwargs` - A `HashMap<String, Value>` containing additional keyword arguments for decoding.
///
/// # Returns
/// A `Result<Vec<Value>, Box<dyn Error>>` containing either the generated completion(s) as a vector of `Value`s or an error.
///
/// # Errors
/// This function will return an error if the API request fails, or if there is a problem with the input arguments.
pub async fn openai_completion(
    prompt: Value,
    decoding_args: OpenAIDecodingArguments,
    model_name: &str,
    sleep_time: u64,
    batch_size: usize,
    max_instances: usize,
    return_text: bool,
    decoding_kwargs: HashMap<String, Value>,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let single_prompt = is_single_prompt(&prompt);
    let prompts = if single_prompt {
        vec![prompt]
    } else {
        prompt.as_array().unwrap().clone()
    };

    let prompt_batches = prepare_prompt_batches(prompts, batch_size, max_instances);
    let client = Client::new();
    let url = format!(
        "https://api.openai.com/v1/engines/{}/completions",
        model_name
    );
    let api_key = "your_openai_api_key"; // Replace with your OpenAI API key.
    let mut completions = Vec::new();

    for (batch_id, prompt_batch) in prompt_batches.into_iter().enumerate() {
        let mut batch_decoding_args = decoding_args.clone();
        let mut success = false;

        while !success {
            match send_request(
                &client,
                &url,
                api_key,
                &prompt_batch,
                &batch_decoding_args,
                &decoding_kwargs,
            )
            .await
            {
                Ok(choices) => {
                    completions.extend(choices);
                    success = true;
                }
                Err(err) => {
                    eprintln!("OpenAIError: {}", err);

                    if err.to_string().contains("Please reduce your prompt") {
                        batch_decoding_args.max_tokens =
                            (batch_decoding_args.max_tokens as f64 * 0.8) as u32;
                        eprintln!(
                            "Reducing target length to {}, Retrying...",
                            batch_decoding_args.max_tokens
                        );
                    } else {
                        eprintln!("Hit request rate limit; retrying...");
                        thread::sleep(Duration::from_secs(sleep_time));
                    }
                }
            }
        }
    }

    if return_text {
        completions = completions
            .into_iter()
            .map(|completion| completion["text"].clone())
            .collect();
    }

    if decoding_args.n > 1 {
        let n = decoding_args.n as usize;
        completions = completions
            .chunks(n)
            .map(|chunk| Value::Array(chunk.to_vec()))
            .collect();
    }

    if single_prompt {
        let (completion,) = (completions[0].clone(),);
        completions = vec![completion];
    }

    Ok(completions)
}

/// Create a BufWriter for the file.
///
/// If the input is a file path, it opens the file in the specified mode.
/// If the file does not exist, it creates the file and its parent directories if necessary.
///
/// # Arguments
///
/// * `path` - A file path or a file object to be written to.
/// * `mode` - Mode for opening the file, either "write" or "append".
fn make_w_io_base<P: AsRef<Path>>(
    path: P,
    mode: &str,
) -> Result<BufWriter<File>, Box<dyn std::error::Error>> {
    let file = match mode {
        "write" => OpenOptions::new().write(true).create(true).open(path)?,
        "append" => OpenOptions::new().append(true).create(true).open(path)?,
        _ => panic!("Invalid mode specified: {}", mode),
    };

    Ok(BufWriter::new(file))
}

/// Create a BufReader for the file.
///
/// If the input is a file path, it opens the file in read mode.
///
/// # Arguments
///
/// * `path` - A file path or a file object to be read from.
fn make_r_io_base<P: AsRef<Path>>(path: P) -> Result<BufReader<File>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    Ok(BufReader::new(file))
}

/// Dump a serializable object to a file in JSON format.
///
/// # Arguments
///
/// * `obj` - An object to be written.
/// * `path` - A file path or a file object to be written to.
/// * `mode` - Mode for opening the file, either "write" or "append".
/// * `indent` - Indent for storing JSON objects.
pub fn jdump<T: Serialize, P: AsRef<Path>>(
    obj: &T,
    path: P,
    mode: &str,
    indent: Option<usize>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = make_w_io_base(path, mode)?;
    let indent_str = " ".repeat(indent.unwrap_or(4));
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent_str.as_bytes());
    let mut serializer = serde_json::Serializer::with_formatter(&mut writer, formatter);
    obj.serialize(&mut serializer)?;
    Ok(())
}

/// Load a JSON file into a deserializable object.
///
/// # Arguments
///
/// * `path` - A file path or a file object to be read from.
pub fn jload<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(
    path: P,
) -> Result<T, Box<dyn std::error::Error>> {
    let reader = make_r_io_base(path)?;
    let obj = serde_json::from_reader(reader)?;
    Ok(obj)
}
