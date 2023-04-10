use kakashi::generate_instruction::encode_prompt;

fn main() {
    let prompt_instructions = vec![
        (
            "Translate the following sentence into French:",
            "The weather is nice today.",
            "Il fait beau aujourd'hui.",
        ),
        ("Find the square root of the following number:", "144", "12"),
    ];
    let prompt = encode_prompt(&prompt_instructions).unwrap_or_else(|err| {
        eprintln!("Error occurred: {}", err);
        std::process::exit(1);
    });
    println!("{}", prompt);
}
