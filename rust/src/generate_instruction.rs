use regex::Regex;
use std::fs::File;
use std::io::{prelude::*, Error};

pub fn encode_prompt(prompt_instructions: &Vec<(&str, &str, &str)>) -> Result<String, Error> {
    let mut prompt = String::new();
    {
        let mut file = File::open("prompt.txt").map_err(|e| {
            eprintln!("Error: Unable to open the file");
            e
        })?;
        file.read_to_string(&mut prompt).map_err(|e| {
            eprintln!("Error: Unable to read the file");
            e
        })?;
    }
    prompt.push('\n');

    for (idx, (instruction, input, output)) in prompt_instructions.iter().enumerate() {
        let instruction = {
            let re = Regex::new(r"\s+").unwrap();
            let instruction = re.replace_all(instruction, " ");
            instruction.trim().trim_end_matches(':').to_string()
        };
        let input = if input.to_lowercase().is_empty() {
            "<noinput>".to_string()
        } else {
            input.to_string()
        };
        prompt.push_str(&format!("###\n"));
        prompt.push_str(&format!("{}. Instruction: {}\n", idx + 1, instruction));
        prompt.push_str(&format!("{}. Input:\n{}\n", idx + 1, input));
        prompt.push_str(&format!("{}. Output:\n{}\n", idx + 1, output));
    }
    prompt.push_str(&format!("###\n"));
    prompt.push_str(&format!("{}. Instruction:", prompt_instructions.len() + 1));

    Ok(prompt)
}
