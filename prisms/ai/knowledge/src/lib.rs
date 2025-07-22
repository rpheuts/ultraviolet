pub mod spectrum;
pub mod storage;

use serde_json::{json, Value};
use uuid::Uuid;
use uv_core::{Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum};

use spectrum::{StoreRequest, SearchRequest, RetrieveRequest, UpdateRequest, ListTagsRequest};
use storage::KnowledgeStorage;

pub struct AIKnowledgePrism {
    spectrum: Option<UVSpectrum>,
    storage: KnowledgeStorage,
}

impl AIKnowledgePrism {
    pub fn new() -> Result<Self> {
        let storage = KnowledgeStorage::new()?;
        
        Ok(Self {
            spectrum: None,
            storage,
        })
    }
    
    fn handle_store(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: StoreRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid store request: {}", e)))?;
        
        let entry = self.storage.store(
            request.content,
            request.title,
            request.tags,
            request.category,
        )?;
        
        let response = json!({
            "id": entry.id,
            "stored_at": entry.created_at.to_rfc3339()
        });
        
        link.emit_photon(id, response)?;
        link.emit_trap(id, None)?;
        Ok(())
    }
    
    fn handle_search(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: SearchRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid search request: {}", e)))?;
        
        let results = self.storage.search(
            &request.query,
            request.tags.as_deref(),
            request.category.as_deref(),
            request.limit,
        )?;
        
        for result in results {
            let response = json!({
                "id": result.id,
                "title": result.title,
                "content": result.content,
                "tags": result.tags,
                "category": result.category,
                "relevance_score": result.relevance_score
            });

            link.emit_photon(id, response)?;
        }
        
        link.emit_trap(id, None)?;
        Ok(())
    }
    
    fn handle_retrieve(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: RetrieveRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid retrieve request: {}", e)))?;
        
        let entry = self.storage.retrieve(&request.id)?;
        
        let response = json!({
            "id": entry.id,
            "title": entry.title,
            "content": entry.content,
            "tags": entry.tags,
            "category": entry.category,
            "created_at": entry.created_at.to_rfc3339(),
            "updated_at": entry.updated_at.to_rfc3339()
        });
        
        link.emit_photon(id, response)?;
        link.emit_trap(id, None)?;
        Ok(())
    }
    
    fn handle_update(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: UpdateRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid update request: {}", e)))?;
        
        let entry = self.storage.update(
            &request.id,
            request.content,
            request.title,
            request.tags,
            request.category,
        )?;
        
        let response = json!({
            "id": entry.id,
            "updated_at": entry.updated_at.to_rfc3339()
        });
        
        link.emit_photon(id, response)?;
        link.emit_trap(id, None)?;
        Ok(())
    }
    
    fn handle_list_categories(&self, id: Uuid, _input: Value, link: &UVLink) -> Result<()> {
        let categories = self.storage.list_categories()?;
        let response = json!({ "categories": categories });
        
        link.emit_photon(id, response)?;
        link.emit_trap(id, None)?;
        Ok(())
    }
    
    fn handle_list_tags(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        let request: ListTagsRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid list_tags request: {}", e)))?;
        
        let tags = self.storage.list_tags(request.prefix.as_deref())?;
        let response = json!({ "tags": tags });
        
        link.emit_photon(id, response)?;
        link.emit_trap(id, None)?;
        Ok(())
    }
}

impl UVPrism for AIKnowledgePrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "store" => {
                    self.handle_store(id, wavefront.input.clone(), link)?;
                    Ok(true)
                },
                "search" => {
                    self.handle_search(id, wavefront.input.clone(), link)?;
                    Ok(true)
                },
                "retrieve" => {
                    self.handle_retrieve(id, wavefront.input.clone(), link)?;
                    Ok(true)
                },
                "update" => {
                    self.handle_update(id, wavefront.input.clone(), link)?;
                    Ok(true)
                },
                "list_categories" => {
                    self.handle_list_categories(id, wavefront.input.clone(), link)?;
                    Ok(true)
                },
                "list_tags" => {
                    self.handle_list_tags(id, wavefront.input.clone(), link)?;
                    Ok(true)
                },
                _ => {
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    Ok(true)
                }
            }
        } else {
            Ok(false)
        }
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(AIKnowledgePrism::new().unwrap())
}