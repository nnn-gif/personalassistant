use crate::error::{AppError, Result};
use chrono::{DateTime, Utc};
use mime_guess::{mime, MimeGuess};
use pdf_extract;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProcessedDocument {
    pub title: String,
    pub content: String,
    pub file_type: String,
    pub metadata: HashMap<String, String>,
    pub language: Option<String>,
    pub word_count: usize,
    pub char_count: usize,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    pub file_name: String,
    pub file_path: String,
    pub file_type: String,
    pub file_size: u64,
    pub modified_time: Option<DateTime<Utc>>,
    pub created_time: Option<DateTime<Utc>>,
    pub encoding: Option<String>,
    pub language: Option<String>,
    pub author: Option<String>,
    pub title: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
}

pub struct EnhancedDocumentProcessor {
    // Configuration
    max_file_size: u64,
    max_content_length: usize,
    extract_metadata: bool,
    detect_language: bool,

    // Regex patterns for cleaning
    cleanup_patterns: Vec<Regex>,

    // Supported formats
    supported_extensions: Vec<&'static str>,
}

impl EnhancedDocumentProcessor {
    pub fn new() -> Self {
        Self {
            max_file_size: 100 * 1024 * 1024,     // 100MB
            max_content_length: 10 * 1024 * 1024, // 10MB text
            extract_metadata: true,
            detect_language: true,
            cleanup_patterns: Self::init_cleanup_patterns(),
            supported_extensions: vec![
                // Text files
                "txt",
                "md",
                "markdown",
                "rst",
                "asciidoc",
                "org",
                // Programming languages
                "rs",
                "py",
                "js",
                "ts",
                "go",
                "java",
                "cpp",
                "c",
                "h",
                "hpp",
                "cs",
                "php",
                "rb",
                "swift",
                "kt",
                "scala",
                "clj",
                "hs",
                "ml",
                "r",
                "jl",
                "dart",
                "lua",
                "perl",
                "sh",
                "bash",
                "zsh",
                "fish",
                // Web technologies
                "html",
                "htm",
                "xml",
                "css",
                "scss",
                "sass",
                "less",
                "jsx",
                "tsx",
                "vue",
                // Configuration files
                "json",
                "yaml",
                "yml",
                "toml",
                "ini",
                "cfg",
                "conf",
                "config",
                "properties",
                "env",
                "dotenv",
                // Documents
                "pdf",
                "docx",
                "doc",
                "odt",
                "rtf",
                // Data files
                "csv",
                "tsv",
                "sql",
                "log",
                "jsonl",
                "ndjson",
                // Other
                "dockerfile",
                "makefile",
                "license",
                "readme",
                "changelog",
                "gitignore",
                "gitattributes",
                "editorconfig",
            ],
        }
    }

    pub fn with_config(mut self, max_file_size: u64, max_content_length: usize) -> Self {
        self.max_file_size = max_file_size;
        self.max_content_length = max_content_length;
        self
    }

    fn init_cleanup_patterns() -> Vec<Regex> {
        vec![
            // Remove excessive whitespace
            Regex::new(r"\s+").unwrap(),
            // Remove control characters except newlines and tabs
            Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap(),
            // Remove zero-width characters
            Regex::new(r"[\u200B-\u200D\uFEFF]").unwrap(),
            // Remove PDF artifacts
            Regex::new(r"Identity-H\s+Unimplemented").unwrap(),
            // Remove repeated dashes/underscores
            Regex::new(r"[-_]{10,}").unwrap(),
            // Remove page numbers (common patterns)
            Regex::new(r"(?m)^\s*\d+\s*$").unwrap(),
            // Remove lines that are mostly repeated characters (dashes, underscores, equals, etc.)
            Regex::new(r"(?m)^[-_=~*+]{10,}$").unwrap(),
        ]
    }

    pub async fn process_file(&self, file_path: &str) -> Result<ProcessedDocument> {
        let start_time = std::time::Instant::now();

        println!("📄 Starting document processing for: {file_path}");

        let path = Path::new(file_path);

        // Validate file
        println!("🔍 Validating file: {file_path}");
        self.validate_file(path)?;
        println!("✅ File validation passed");

        // Extract metadata
        println!("📊 Extracting file metadata...");
        let metadata = self.extract_file_metadata(path)?;
        println!(
            "✅ Metadata extracted - Size: {} bytes, Type: {}",
            metadata.file_size, metadata.file_type
        );

        let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
        println!("🔍 Detected MIME type: {mime_type}");

        // Extract content
        println!("📝 Extracting content from file...");
        let content = self.extract_content(path, &mime_type).await?;
        println!(
            "✅ Content extracted - Raw length: {} characters",
            content.len()
        );

        // Clean content
        println!("🧹 Cleaning content...");
        let cleaned_content = self.clean_content(&content);
        println!(
            "✅ Content cleaned - Final length: {} characters",
            cleaned_content.len()
        );

        // Detect language
        let language = if self.detect_language {
            println!("🔍 Detecting language...");
            let detected = self.detect_language(&cleaned_content);
            if let Some(ref lang) = detected {
                println!("✅ Language detected: {lang}");
            } else {
                println!("❓ Language detection: unknown");
            }
            detected
        } else {
            println!("⏭️  Language detection skipped");
            None
        };

        let processing_time = start_time.elapsed().as_millis() as u64;

        let word_count = cleaned_content.split_whitespace().count();
        let char_count = cleaned_content.chars().count();

        println!("📈 Processing statistics:");
        println!("   - Processing time: {processing_time}ms");
        println!("   - Word count: {word_count}");
        println!("   - Character count: {char_count}");

        let mut doc_metadata = HashMap::new();
        doc_metadata.insert("file_name".to_string(), metadata.file_name.clone());
        doc_metadata.insert("file_path".to_string(), metadata.file_path.clone());
        doc_metadata.insert("file_type".to_string(), metadata.file_type.clone());
        doc_metadata.insert("file_size".to_string(), metadata.file_size.to_string());
        doc_metadata.insert("word_count".to_string(), word_count.to_string());
        doc_metadata.insert("char_count".to_string(), char_count.to_string());
        doc_metadata.insert(
            "processing_time_ms".to_string(),
            processing_time.to_string(),
        );

        if let Some(modified) = metadata.modified_time {
            doc_metadata.insert("modified_time".to_string(), modified.to_rfc3339());
        }

        if let Some(lang) = &language {
            doc_metadata.insert("language".to_string(), lang.clone());
        }

        if let Some(author) = metadata.author {
            doc_metadata.insert("author".to_string(), author);
        }

        if let Some(title) = metadata.title {
            doc_metadata.insert("document_title".to_string(), title);
        }

        if !metadata.keywords.is_empty() {
            doc_metadata.insert("keywords".to_string(), metadata.keywords.join(", "));
        }

        println!("📋 Building final document structure...");
        let title = Self::generate_title(&metadata.file_name, &cleaned_content);
        println!("📝 Generated title: {title}");

        let processed_doc = ProcessedDocument {
            title,
            content: cleaned_content,
            file_type: mime_type.to_string(),
            metadata: doc_metadata,
            language,
            word_count,
            char_count,
            processing_time_ms: processing_time,
        };

        println!("🎉 Document processing completed successfully!");
        println!("📄 Final document summary:");
        println!("   - Title: {}", processed_doc.title);
        println!("   - File type: {}", processed_doc.file_type);
        println!(
            "   - Language: {}",
            processed_doc.language.as_deref().unwrap_or("unknown")
        );
        println!(
            "   - Processing time: {}ms",
            processed_doc.processing_time_ms
        );
        println!(
            "   - Content length: {} characters",
            processed_doc.content.len()
        );
        println!("   - Word count: {}", processed_doc.word_count);
        println!("   - Character count: {}", processed_doc.char_count);

        Ok(processed_doc)
    }

    fn validate_file(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Err(AppError::NotFound(format!(
                "File not found: {}",
                path.display()
            )));
        }

        if !path.is_file() {
            return Err(AppError::InvalidInput(format!(
                "Path is not a file: {}",
                path.display()
            )));
        }

        let metadata = fs::metadata(path)
            .map_err(|e| AppError::ProcessingError(format!("Cannot read file metadata: {e}")))?;

        if metadata.len() > self.max_file_size {
            return Err(AppError::ProcessingError(format!(
                "File too large: {} bytes (max: {} bytes)",
                metadata.len(),
                self.max_file_size
            )));
        }

        if metadata.len() == 0 {
            return Err(AppError::ProcessingError("File is empty".to_string()));
        }

        Ok(())
    }

    fn extract_file_metadata(&self, path: &Path) -> Result<DocumentMetadata> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path = path.to_string_lossy().to_string();
        let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
        let file_type = mime_type.to_string();

        let fs_metadata = fs::metadata(path)
            .map_err(|e| AppError::ProcessingError(format!("Cannot read file metadata: {e}")))?;

        let file_size = fs_metadata.len();

        let modified_time = fs_metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, 0));

        let created_time = fs_metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, 0));

        Ok(DocumentMetadata {
            file_name,
            file_path,
            file_type,
            file_size,
            modified_time,
            created_time,
            encoding: None,
            language: None,
            author: None,
            title: None,
            subject: None,
            keywords: Vec::new(),
        })
    }

    async fn extract_content(&self, path: &Path, mime_type: &mime::Mime) -> Result<String> {
        println!("🔧 Processing content based on MIME type: {mime_type}");

        match mime_type.type_() {
            mime::TEXT => {
                println!("📄 Processing as text file");
                self.process_text_file(path).await
            }
            mime::APPLICATION => {
                match mime_type.subtype().as_str() {
                    "pdf" => {
                        println!("📑 Processing as PDF file");
                        self.process_pdf_file(path).await
                    }
                    "vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                        println!("📄 Processing as DOCX file");
                        self.process_docx_file(path).await
                    }
                    "msword" => {
                        println!("📄 Processing as legacy DOC file");
                        self.process_doc_file(path).await
                    }
                    "json" => {
                        println!("📄 Processing as JSON file");
                        self.process_json_file(path).await
                    }
                    "xml" => {
                        println!("📄 Processing as XML file");
                        self.process_xml_file(path).await
                    }
                    _ => {
                        println!(
                            "❓ Unknown application type ({}), attempting text processing",
                            mime_type.subtype()
                        );
                        // Try to read as text for unknown application types
                        self.process_text_file(path).await.or_else(|_| {
                            println!("❌ Text processing failed, treating as binary file");
                            Ok(format!(
                                "Binary file ({}): {}",
                                mime_type.subtype(),
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ))
                        })
                    }
                }
            }
            _ => {
                println!(
                    "❓ Unsupported MIME type ({}), attempting text processing",
                    mime_type
                );
                // Try to read as text for other types
                self.process_text_file(path).await.or_else(|_| {
                    println!("❌ Text processing failed, treating as unsupported file");
                    Ok(format!(
                        "Unsupported file type ({}): {}",
                        mime_type,
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ))
                })
            }
        }
    }

    async fn process_text_file(&self, path: &Path) -> Result<String> {
        println!("📖 Reading text file...");
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::ProcessingError(format!("Error reading text file: {e}")))?;

        println!("📊 Text file read - Length: {} characters", content.len());

        if content.len() > self.max_content_length {
            println!(
                "⚠️  Content length exceeds maximum: {} > {}",
                content.len(),
                self.max_content_length
            );
            return Err(AppError::ProcessingError(format!(
                "Text content too large: {} chars (max: {} chars)",
                content.len(),
                self.max_content_length
            )));
        }

        println!("✅ Text file processing completed");
        Ok(content)
    }

    async fn process_pdf_file(&self, path: &Path) -> Result<String> {
        use lopdf::Document;

        println!("📑 Loading PDF document...");
        let doc = Document::load(path)
            .map_err(|e| AppError::ProcessingError(format!("Error loading PDF: {e}")))?;

        let mut text = String::new();
        let pages = doc.get_pages();

        println!("📄 PDF has {} pages", pages.len());

        // Extract text from each page
        for (page_num, _) in pages.iter().enumerate() {
            println!("📖 Processing page {}/{}", page_num + 1, pages.len());
            match doc.extract_text(&[(page_num + 1) as u32]) {
                Ok(page_text) => {
                    println!(
                        "✅ Page {} extracted - {} characters",
                        page_num + 1,
                        page_text.len()
                    );

                    // Check if extraction is mostly question marks (indicates encoding issues)
                    let question_mark_ratio = if page_text.len() > 0 {
                        page_text.chars().filter(|&c| c == '?').count() as f32
                            / page_text.len() as f32
                    } else {
                        0.0
                    };

                    // Show actual extracted content for debugging
                    println!(
                        "📄 Raw text sample (first 200 chars): {}",
                        page_text.chars().take(200).collect::<String>()
                    );

                    // Check for specific lopdf encoding errors
                    if page_text.contains("Identity-H Unimplemented")
                        || page_text.contains("Identity-V Unimplemented")
                    {
                        println!("❌ PDF page {} contains Identity-H/V encoding - lopdf library limitation", page_num + 1);
                        println!("💡 This PDF uses CID/CMap encoding that lopdf cannot decode");
                        println!("🔄 Attempting fallback to pdf-extract library...");

                        // Try pdf-extract as fallback
                        match self.extract_pdf_with_fallback(path.to_str().unwrap()).await {
                            Ok(fallback_text) => {
                                if !fallback_text.trim().is_empty() {
                                    println!(
                                        "✅ pdf-extract fallback successful - {} characters",
                                        fallback_text.len()
                                    );
                                    text.push_str(&fallback_text);
                                    text.push('\n');
                                } else {
                                    println!(
                                        "❌ pdf-extract fallback also failed - no readable text"
                                    );
                                }
                            }
                            Err(e) => {
                                println!("❌ pdf-extract fallback failed: {e}");
                            }
                        }
                        continue;
                    }

                    if question_mark_ratio > 0.5 {
                        println!(
                            "⚠️  PDF page {} has {:.1}% question marks - likely encoding/OCR issue",
                            page_num + 1,
                            question_mark_ratio * 100.0
                        );
                        println!("❌ Skipping page {} due to extraction issues", page_num + 1);
                        continue;
                    } else if question_mark_ratio > 0.1 {
                        println!("⚠️  PDF page {} has {:.1}% question marks - may have some encoding issues", page_num + 1, question_mark_ratio * 100.0);
                    }

                    let cleaned_page = self.clean_pdf_text(&page_text);
                    if !cleaned_page.trim().is_empty() {
                        text.push_str(&cleaned_page);
                        text.push('\n');
                        println!(
                            "✅ Page {} cleaned - {} characters",
                            page_num + 1,
                            cleaned_page.len()
                        );
                    } else {
                        println!(
                            "⚠️  Page {} contains no readable text after cleaning",
                            page_num + 1
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "❌ Warning: Could not extract text from page {}: {}",
                        page_num + 1,
                        e
                    );
                }
            }
        }

        if text.trim().is_empty() {
            println!("❌ No readable text found in PDF");
            return Err(AppError::ProcessingError(
                "No readable text found in PDF".to_string(),
            ));
        }

        println!(
            "✅ PDF processing completed - Total text: {} characters",
            text.len()
        );
        Ok(text)
    }

    async fn extract_pdf_with_fallback(&self, file_path: &str) -> Result<String> {
        println!("🔄 Using pdf-extract as fallback for: {file_path}");

        // Use pdf-extract which handles more PDF formats
        let text = pdf_extract::extract_text(file_path)
            .map_err(|e| AppError::ProcessingError(format!("pdf-extract failed: {e}")))?;

        println!("✅ pdf-extract extracted {} characters", text.len());

        // Clean the extracted text
        let cleaned = self.clean_pdf_text(&text);

        Ok(cleaned)
    }

    fn clean_pdf_text(&self, text: &str) -> String {
        let mut cleaned = text.to_string();

        // Remove common PDF artifacts
        cleaned = cleaned.replace("Identity-H Unimplemented", "");
        cleaned = cleaned.replace("cid:", "");

        // Remove excessive whitespace
        cleaned = Regex::new(r"\s+")
            .unwrap()
            .replace_all(&cleaned, " ")
            .to_string();

        // Remove isolated single characters that are likely artifacts
        cleaned = Regex::new(r"\b[a-zA-Z]\b")
            .unwrap()
            .replace_all(&cleaned, "")
            .to_string();

        // Remove lines with only numbers (likely page numbers)
        cleaned = Regex::new(r"(?m)^\s*\d+\s*$")
            .unwrap()
            .replace_all(&cleaned, "")
            .to_string();

        cleaned.trim().to_string()
    }

    async fn process_docx_file(&self, path: &Path) -> Result<String> {
        println!("📄 Opening DOCX file...");
        let file = std::fs::File::open(path)
            .map_err(|e| AppError::ProcessingError(format!("Error opening DOCX file: {e}")))?;

        println!("📦 Reading DOCX archive...");
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::ProcessingError(format!("Error reading DOCX archive: {e}")))?;

        // Read document.xml
        println!("📄 Extracting document.xml...");
        let mut document_xml = archive
            .by_name("word/document.xml")
            .map_err(|e| AppError::ProcessingError(format!("Error reading document.xml: {e}")))?;

        println!("📖 Reading XML content...");
        let mut xml_content = String::new();
        document_xml
            .read_to_string(&mut xml_content)
            .map_err(|e| AppError::ProcessingError(format!("Error reading XML content: {e}")))?;

        println!("📊 XML content read - {} characters", xml_content.len());

        println!("🔍 Parsing XML and extracting text...");
        let text = self.extract_text_from_docx_xml(&xml_content)?;

        if text.trim().is_empty() {
            println!("❌ No text found in DOCX");
            return Err(AppError::ProcessingError(
                "No text found in DOCX".to_string(),
            ));
        }

        println!("✅ DOCX processing completed - {} characters", text.len());
        Ok(text)
    }

    fn extract_text_from_docx_xml(&self, xml_content: &str) -> Result<String> {
        // More sophisticated XML parsing for DOCX
        let mut text = String::new();
        let mut in_text_tag = false;
        let mut current_text = String::new();
        let mut chars = xml_content.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '<' => {
                    if in_text_tag {
                        text.push_str(&current_text);
                        current_text.clear();
                    }

                    // Read tag name
                    let mut tag = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '>' || next_ch == ' ' {
                            break;
                        }
                        tag.push(chars.next().unwrap());
                    }

                    // Skip to end of tag
                    while let Some(next_ch) = chars.next() {
                        if next_ch == '>' {
                            break;
                        }
                    }

                    // Check if this is a text tag
                    in_text_tag = tag == "w:t";

                    // Add paragraph breaks
                    if tag == "w:p" {
                        text.push('\n');
                    }
                }
                _ if in_text_tag => {
                    current_text.push(ch);
                }
                _ => {}
            }
        }

        if in_text_tag {
            text.push_str(&current_text);
        }

        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    async fn process_doc_file(&self, _path: &Path) -> Result<String> {
        // For .doc files, we would need a more complex parser
        // For now, return an informative message
        Err(AppError::ProcessingError(
            "Legacy .doc files are not supported. Please convert to .docx".to_string(),
        ))
    }

    async fn process_json_file(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::ProcessingError(format!("Error reading JSON file: {e}")))?;

        // Pretty-print JSON for better readability
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(json) => {
                let pretty = serde_json::to_string_pretty(&json).unwrap_or(content);
                Ok(pretty)
            }
            Err(_) => {
                // If not valid JSON, return as-is
                Ok(content)
            }
        }
    }

    async fn process_xml_file(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::ProcessingError(format!("Error reading XML file: {e}")))?;

        // For XML files, we might want to extract just the text content
        // For now, return the full XML
        Ok(content)
    }

    fn clean_content(&self, content: &str) -> String {
        println!("🧹 Starting content cleaning process...");
        let mut cleaned = content.to_string();
        let original_length = cleaned.len();

        // Apply cleanup patterns
        println!(
            "🔧 Applying {} cleanup patterns...",
            self.cleanup_patterns.len()
        );
        for (i, pattern) in self.cleanup_patterns.iter().enumerate() {
            let before_len = cleaned.len();
            cleaned = pattern.replace_all(&cleaned, " ").to_string();
            let after_len = cleaned.len();
            if before_len != after_len {
                println!(
                    "   Pattern {} applied: {} -> {} characters",
                    i + 1,
                    before_len,
                    after_len
                );
            }
        }

        // Remove excessive newlines
        println!("🔧 Removing excessive newlines...");
        let before_newlines = cleaned.len();
        cleaned = Regex::new(r"\n\s*\n\s*\n")
            .unwrap()
            .replace_all(&cleaned, "\n\n")
            .to_string();
        if before_newlines != cleaned.len() {
            println!(
                "   Newline cleanup: {} -> {} characters",
                before_newlines,
                cleaned.len()
            );
        }

        // Trim and normalize whitespace
        println!("🔧 Normalizing whitespace...");
        let before_normalize = cleaned.len();
        let lines: Vec<&str> = cleaned
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();
        cleaned = lines.join("\n");

        let final_length = cleaned.trim().len();
        cleaned = cleaned.trim().to_string();

        println!("✅ Content cleaning completed:");
        println!("   Original: {original_length} characters");
        println!("   Final: {final_length} characters");
        println!(
            "   Reduction: {} characters ({:.1}%)",
            original_length - final_length,
            ((original_length - final_length) as f64 / original_length as f64) * 100.0
        );

        cleaned
    }

    fn detect_language(&self, content: &str) -> Option<String> {
        // Simple language detection based on common patterns
        // In a real implementation, you might use a proper language detection library

        let content_lower = content.to_lowercase();

        // Check for common programming languages
        if content_lower.contains("fn main()") || content_lower.contains("use std::") {
            return Some("rust".to_string());
        }
        if content_lower.contains("def ") && content_lower.contains("import ") {
            return Some("python".to_string());
        }
        if content_lower.contains("function ") || content_lower.contains("const ") {
            return Some("javascript".to_string());
        }
        if content_lower.contains("public class ") || content_lower.contains("import java.") {
            return Some("java".to_string());
        }

        // Check for markup languages
        if content_lower.contains("<!doctype html>") || content_lower.contains("<html") {
            return Some("html".to_string());
        }
        if content_lower.contains("<?xml") {
            return Some("xml".to_string());
        }

        // Default to text
        Some("text".to_string())
    }

    fn generate_title(file_name: &str, content: &str) -> String {
        // Try to extract a meaningful title from the content
        let lines: Vec<&str> = content.lines().collect();

        // Look for markdown headers
        for line in lines.iter().take(10) {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                return trimmed.trim_start_matches("# ").trim().to_string();
            }
        }

        // Look for HTML titles
        if let Some(title_match) = Regex::new(r"<title>(.*?)</title>")
            .unwrap()
            .captures(content)
        {
            if let Some(title) = title_match.get(1) {
                return title.as_str().trim().to_string();
            }
        }

        // Use first meaningful line as title
        for line in lines.iter().take(5) {
            let trimmed = line.trim();
            if trimmed.len() > 10 && trimmed.len() < 100 {
                return trimmed.to_string();
            }
        }

        // Fallback to filename without extension
        Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file_name)
            .to_string()
    }

    pub fn get_supported_extensions(&self) -> Vec<&'static str> {
        self.supported_extensions.clone()
    }

    pub fn is_supported_file(&self, file_path: &str) -> bool {
        let path = Path::new(file_path);
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            self.supported_extensions
                .contains(&extension.to_lowercase().as_str())
        } else {
            // Check for files without extensions that might be supported
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let lower_name = file_name.to_lowercase();
                matches!(
                    lower_name.as_str(),
                    "dockerfile"
                        | "makefile"
                        | "license"
                        | "readme"
                        | "changelog"
                        | "gitignore"
                        | "gitattributes"
                        | "editorconfig"
                )
            } else {
                false
            }
        }
    }

    pub fn get_file_stats(&self, file_path: &str) -> Result<HashMap<String, String>> {
        let path = Path::new(file_path);
        let metadata = fs::metadata(path)
            .map_err(|e| AppError::ProcessingError(format!("Cannot read file metadata: {e}")))?;

        let mut stats = HashMap::new();
        stats.insert("size".to_string(), metadata.len().to_string());
        stats.insert("is_file".to_string(), metadata.is_file().to_string());
        stats.insert("is_dir".to_string(), metadata.is_dir().to_string());
        stats.insert(
            "readonly".to_string(),
            metadata.permissions().readonly().to_string(),
        );

        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                if let Some(datetime) = DateTime::from_timestamp(duration.as_secs() as i64, 0) {
                    stats.insert("modified".to_string(), datetime.to_rfc3339());
                }
            }
        }

        Ok(stats)
    }
}

impl Default for EnhancedDocumentProcessor {
    fn default() -> Self {
        Self::new()
    }
}
