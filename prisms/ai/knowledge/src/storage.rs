use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use chrono::Utc;
use uuid::Uuid;
use uv_core::{Result, UVError};
use crate::spectrum::{KnowledgeEntry, SearchResult};

pub struct KnowledgeStorage {
    storage_dir: PathBuf,
}

impl KnowledgeStorage {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| UVError::ExecutionError("Could not find home directory".to_string()))?;
        
        let storage_dir = home_dir.join(".uv").join("knowledge");
        
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir)
                .map_err(|e| UVError::ExecutionError(format!("Failed to create storage directory: {}", e)))?;
        }
        
        Ok(Self { storage_dir })
    }
    
    pub fn store(&self, content: String, title: Option<String>, tags: Option<Vec<String>>, category: Option<String>) -> Result<KnowledgeEntry> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let entry = KnowledgeEntry {
            id: id.clone(),
            title,
            content,
            tags: tags.unwrap_or_default(),
            category,
            created_at: now,
            updated_at: now,
        };
        
        let file_path = self.storage_dir.join(format!("{}.toml", id));
        let toml_content = toml::to_string_pretty(&entry)
            .map_err(|e| UVError::ExecutionError(format!("Failed to serialize entry: {}", e)))?;
        
        fs::write(&file_path, toml_content)
            .map_err(|e| UVError::ExecutionError(format!("Failed to write entry: {}", e)))?;
        
        Ok(entry)
    }
    
    pub fn retrieve(&self, id: &str) -> Result<KnowledgeEntry> {
        let file_path = self.storage_dir.join(format!("{}.toml", id));
        
        if !file_path.exists() {
            return Err(UVError::ExecutionError(format!("Knowledge entry {} not found", id)));
        }
        
        let content = fs::read_to_string(&file_path)
            .map_err(|e| UVError::ExecutionError(format!("Failed to read entry: {}", e)))?;
        
        let entry: KnowledgeEntry = toml::from_str(&content)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse entry: {}", e)))?;
        
        Ok(entry)
    }
    
    pub fn update(&self, id: &str, content: Option<String>, title: Option<String>, tags: Option<Vec<String>>, category: Option<String>) -> Result<KnowledgeEntry> {
        let mut entry = self.retrieve(id)?;
        
        if let Some(new_content) = content {
            entry.content = new_content;
        }
        if let Some(new_title) = title {
            entry.title = Some(new_title);
        }
        if let Some(new_tags) = tags {
            entry.tags = new_tags;
        }
        if let Some(new_category) = category {
            entry.category = Some(new_category);
        }
        
        entry.updated_at = Utc::now();
        
        let file_path = self.storage_dir.join(format!("{}.toml", id));
        let toml_content = toml::to_string_pretty(&entry)
            .map_err(|e| UVError::ExecutionError(format!("Failed to serialize entry: {}", e)))?;
        
        fs::write(&file_path, toml_content)
            .map_err(|e| UVError::ExecutionError(format!("Failed to write entry: {}", e)))?;
        
        Ok(entry)
    }
    
    pub fn search(&self, query: &str, tag_filter: Option<&[String]>, category_filter: Option<&str>, limit: usize) -> Result<Vec<SearchResult>> {
        let entries = self.load_all_entries()?;
        let query_lower = query.to_lowercase();
        
        let mut results: Vec<SearchResult> = entries
            .into_iter()
            .filter_map(|entry| {
                // Filter by tags
                if let Some(tags) = tag_filter {
                    if !tags.iter().any(|tag| entry.tags.contains(tag)) {
                        return None;
                    }
                }
                
                // Filter by category
                if let Some(category) = category_filter {
                    if entry.category.as_deref() != Some(category) {
                        return None;
                    }
                }
                
                // Calculate relevance score
                let mut score = 0.0;
                
                // Title match (higher weight)
                if let Some(title) = &entry.title {
                    if title.to_lowercase().contains(&query_lower) {
                        score += 2.0;
                    }
                }
                
                // Content match
                if entry.content.to_lowercase().contains(&query_lower) {
                    score += 1.0;
                }
                
                // Tag match
                for tag in &entry.tags {
                    if tag.to_lowercase().contains(&query_lower) {
                        score += 1.5;
                    }
                }
                
                if score > 0.0 {
                    Some(SearchResult {
                        id: entry.id,
                        title: entry.title,
                        content: entry.content,
                        tags: entry.tags,
                        category: entry.category,
                        relevance_score: score,
                    })
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by relevance score (descending)
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        // Limit results
        results.truncate(limit);
        
        Ok(results)
    }
    
    pub fn list_categories(&self) -> Result<Vec<String>> {
        let entries = self.load_all_entries()?;
        let categories: HashSet<String> = entries
            .into_iter()
            .filter_map(|entry| entry.category)
            .collect();
        
        let mut result: Vec<String> = categories.into_iter().collect();
        result.sort();
        Ok(result)
    }
    
    pub fn list_tags(&self, prefix: Option<&str>) -> Result<Vec<String>> {
        let entries = self.load_all_entries()?;
        let mut tags: HashSet<String> = HashSet::new();
        
        for entry in entries {
            for tag in entry.tags {
                if let Some(prefix) = prefix {
                    if tag.starts_with(prefix) {
                        tags.insert(tag);
                    }
                } else {
                    tags.insert(tag);
                }
            }
        }
        
        let mut result: Vec<String> = tags.into_iter().collect();
        result.sort();
        Ok(result)
    }
    
    fn load_all_entries(&self) -> Result<Vec<KnowledgeEntry>> {
        let mut entries = Vec::new();
        
        let dir_entries = fs::read_dir(&self.storage_dir)
            .map_err(|e| UVError::ExecutionError(format!("Failed to read storage directory: {}", e)))?;
        
        for entry in dir_entries {
            let entry = entry
                .map_err(|e| UVError::ExecutionError(format!("Failed to read directory entry: {}", e)))?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match self.load_entry_from_path(&path) {
                    Ok(knowledge_entry) => entries.push(knowledge_entry),
                    Err(_) => continue, // Skip corrupted files
                }
            }
        }
        
        Ok(entries)
    }
    
    fn load_entry_from_path(&self, path: &Path) -> Result<KnowledgeEntry> {
        let content = fs::read_to_string(path)
            .map_err(|e| UVError::ExecutionError(format!("Failed to read file: {}", e)))?;
        
        let entry: KnowledgeEntry = toml::from_str(&content)
            .map_err(|e| UVError::ExecutionError(format!("Failed to parse entry: {}", e)))?;
        
        Ok(entry)
    }
}