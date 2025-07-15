use crate::error::{AppError, Result};
use mime_guess::{mime, MimeGuess};
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ProcessedDocument {
    pub title: String,
    pub content: String,
    pub file_type: String,
    pub metadata: std::collections::HashMap<String, String>,
}

pub struct DocumentProcessor;

impl DocumentProcessor {
    pub fn new() -> Self {
        Self
    }

    pub async fn process_file(&self, file_path: &str) -> Result<ProcessedDocument> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(AppError::NotFound(format!("File not found: {}", file_path)));
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mime_type = MimeGuess::from_path(path).first_or_octet_stream();
        let file_type = mime_type.to_string();

        let content = match mime_type.type_() {
            mime::TEXT => self.process_text_file(path).await?,
            mime::APPLICATION => {
                match mime_type.subtype().as_str() {
                    "pdf" => self.process_pdf_file(path).await?,
                    "vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                        self.process_docx_file(path).await?
                    }
                    _ => {
                        // Try to read as text
                        self.process_text_file(path)
                            .await
                            .unwrap_or_else(|_| format!("Binary file: {}", file_name))
                    }
                }
            }
            _ => {
                // Try to read as text, fallback to file info
                self.process_text_file(path)
                    .await
                    .unwrap_or_else(|_| format!("Unsupported file type: {}", file_name))
            }
        };

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("file_name".to_string(), file_name.clone());
        metadata.insert("file_path".to_string(), file_path.to_string());
        metadata.insert("file_type".to_string(), file_type.clone());

        if let Ok(file_metadata) = fs::metadata(path) {
            if let Ok(modified) = file_metadata.modified() {
                metadata.insert("modified".to_string(), format!("{:?}", modified));
            }
            metadata.insert("size".to_string(), file_metadata.len().to_string());
        }

        Ok(ProcessedDocument {
            title: file_name,
            content,
            file_type,
            metadata,
        })
    }

    async fn process_text_file(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path)
            .map_err(|e| AppError::ProcessingError(format!("Error reading text file: {}", e)))
    }

    async fn process_pdf_file(&self, path: &Path) -> Result<String> {
        use lopdf::Document;

        let doc = Document::load(path)
            .map_err(|e| AppError::ProcessingError(format!("Error loading PDF: {}", e)))?;

        let mut text = String::new();

        // Extract text from each page
        for page_num in 1..=doc.get_pages().len() {
            if let Ok(page_text) = doc.extract_text(&[page_num as u32]) {
                println!("Extracted text from page {}: {}", page_num, page_text);
                text.push_str(&page_text);
                text.push('\n');
            }
        }

        if text.is_empty() {
            return Err(AppError::ProcessingError(
                "No text found in PDF".to_string(),
            ));
        }

        Ok(text)
    }

    async fn process_docx_file(&self, path: &Path) -> Result<String> {
        // For now, we'll use a simplified approach for DOCX files
        // In production, you'd want to use a proper DOCX parser

        // Try to extract as ZIP and read document.xml
        let file = std::fs::File::open(path)
            .map_err(|e| AppError::ProcessingError(format!("Error opening DOCX file: {}", e)))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| AppError::ProcessingError(format!("Error reading DOCX archive: {}", e)))?;

        // Try to read document.xml
        let mut document_xml = archive
            .by_name("word/document.xml")
            .map_err(|e| AppError::ProcessingError(format!("Error reading document.xml: {}", e)))?;

        let mut xml_content = String::new();
        std::io::Read::read_to_string(&mut document_xml, &mut xml_content)
            .map_err(|e| AppError::ProcessingError(format!("Error reading XML content: {}", e)))?;

        // Simple text extraction from XML (remove tags)
        let text = self.extract_text_from_xml(&xml_content);

        if text.is_empty() {
            return Err(AppError::ProcessingError(
                "No text found in DOCX".to_string(),
            ));
        }

        Ok(text)
    }

    fn extract_text_from_xml(&self, xml_content: &str) -> String {
        // Simple XML tag removal - in production use a proper XML parser
        let mut text = String::new();
        let mut in_tag = false;

        for char in xml_content.chars() {
            match char {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => text.push(c),
                _ => {}
            }
        }

        // Clean up whitespace
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn get_supported_extensions(&self) -> Vec<&'static str> {
        vec![
            // Text files
            "txt", "md", "json", "xml", "html", "css", "js", "ts", "py", "rs", "go", "java", "cpp",
            "c", "h", // Documents
            "pdf", "docx", // Code files
            "yml", "yaml", "toml", "ini", "cfg", "conf", // Other
            "log", "csv",
        ]
    }

    pub fn is_supported_file(&self, file_path: &str) -> bool {
        let path = Path::new(file_path);
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            self.get_supported_extensions()
                .contains(&extension.to_lowercase().as_str())
        } else {
            false
        }
    }
}
