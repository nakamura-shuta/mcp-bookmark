use anyhow::{Context, Result};
use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera_tantivy::tokenizer::LinderaTokenizer;
use tantivy::Index;
use tracing::{debug, info};

/// Tokenizer name for Japanese text
pub const JAPANESE_TOKENIZER_NAME: &str = "lang_ja";

/// Register Lindera tokenizer for Japanese text processing
pub fn register_lindera_tokenizer(index: &Index) -> Result<()> {
    debug!("Registering Lindera tokenizer for Japanese text processing");

    // Load IPADIC dictionary
    let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC)
        .context("Failed to load IPADIC dictionary")?;

    // Use Decompose mode for better search results
    let mode = Mode::Decompose(Penalty::default());
    let user_dictionary = None;

    // Create Segmenter with the dictionary
    let segmenter = Segmenter::new(mode, dictionary, user_dictionary);

    // Create Lindera tokenizer from segmenter
    let tokenizer = LinderaTokenizer::from_segmenter(segmenter);

    // Register the tokenizer with name "lang_ja"
    index
        .tokenizers()
        .register(JAPANESE_TOKENIZER_NAME, tokenizer);

    info!("Lindera tokenizer registered successfully");
    Ok(())
}
