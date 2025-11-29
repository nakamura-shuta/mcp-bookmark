use anyhow::{Context, Result};
use lindera::dictionary::{DictionaryKind, load_dictionary_from_kind};
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera_tantivy::tokenizer::LinderaTokenizer;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tantivy::collector::TopDocs;
use tantivy::query::AllQuery;
use tantivy::schema::Value;
use tantivy::{Index, ReloadPolicy};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <index_path> <output_dir>", args[0]);
        eprintln!(
            "Example: {} ~/Library/Application\\ Support/mcp-bookmark/v14_Book ./output",
            args[0]
        );
        std::process::exit(1);
    }

    let index_path = PathBuf::from(&args[1]);
    let output_dir = PathBuf::from(&args[2]);

    // Create output directory
    fs::create_dir_all(&output_dir)?;

    println!("Opening index: {}", index_path.display());

    // Open index
    let index = Index::open_in_dir(&index_path).context("Failed to open index")?;

    // Register Lindera tokenizer
    let dictionary = load_dictionary_from_kind(DictionaryKind::IPADIC)?;
    let mode = Mode::Decompose(Penalty::default());
    let segmenter = Segmenter::new(mode, dictionary, None);
    let tokenizer = LinderaTokenizer::from_segmenter(segmenter);
    index.tokenizers().register("lang_ja", tokenizer);

    let schema = index.schema();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;
    let searcher = reader.searcher();

    // Get all documents
    let top_docs = searcher.search(&AllQuery, &TopDocs::with_limit(10000))?;

    println!("Found {} documents in index", top_docs.len());

    // Create summary file
    let mut summary = File::create(output_dir.join("index_summary.txt"))?;
    writeln!(summary, "Index: {}", index_path.display())?;
    writeln!(summary, "Total documents: {}", top_docs.len())?;
    writeln!(summary)?;

    // Get field handles
    let id_field = schema.get_field("id").ok();
    let url_field = schema.get_field("url").ok();
    let title_field = schema.get_field("title").ok();
    let content_field = schema.get_field("content").ok();
    let page_count_field = schema.get_field("page_count").ok();
    let content_type_field = schema.get_field("content_type").ok();
    let folder_path_field = schema.get_field("folder_path").ok();

    let mut total_content_chars = 0usize;

    for (i, (_score, doc_address)) in top_docs.iter().enumerate() {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_address)?;

        let id = id_field
            .and_then(|f| {
                doc.get_first(f)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        let url = url_field
            .and_then(|f| {
                doc.get_first(f)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        let title = title_field
            .and_then(|f| {
                doc.get_first(f)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        let content = content_field
            .and_then(|f| {
                doc.get_first(f)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        let page_count = page_count_field
            .and_then(|f| doc.get_first(f).and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let content_type = content_type_field
            .and_then(|f| {
                doc.get_first(f)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        let folder_path = folder_path_field
            .and_then(|f| {
                doc.get_first(f)
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();

        let content_chars = content.chars().count();
        total_content_chars += content_chars;

        writeln!(summary, "Document {}: {title}", i + 1)?;
        writeln!(summary, "  ID: {id}")?;
        writeln!(summary, "  URL: {url}")?;
        writeln!(summary, "  Folder: {folder_path}")?;
        writeln!(summary, "  Content length: {content_chars} chars")?;
        writeln!(summary, "  Page count: {page_count}")?;
        writeln!(summary, "  Content type: {content_type}")?;
        writeln!(summary)?;

        // Save content to separate file
        let safe_id = id.replace(['/', ':', '\\', '?', '*'], "_");
        let content_file = output_dir.join(format!("doc_{:03}_{safe_id}.txt", i + 1));
        let mut cf = File::create(&content_file)?;
        writeln!(cf, "Title: {title}")?;
        writeln!(cf, "URL: {url}")?;
        writeln!(cf, "ID: {id}")?;
        writeln!(cf, "Folder: {folder_path}")?;
        writeln!(cf, "Page count: {page_count}")?;
        writeln!(cf, "Content type: {content_type}")?;
        writeln!(cf, "Content length: {content_chars} chars")?;
        writeln!(cf)?;
        writeln!(cf, "=== CONTENT ===")?;
        writeln!(cf, "{content}")?;

        println!(
            "  [{}/{}] {title} ({content_chars} chars)",
            i + 1,
            top_docs.len()
        );
    }

    writeln!(summary, "=== TOTAL ===")?;
    writeln!(summary, "Total content: {total_content_chars} chars")?;

    println!();
    println!("Total content: {total_content_chars} chars");
    println!("Output written to: {}", output_dir.display());

    Ok(())
}
