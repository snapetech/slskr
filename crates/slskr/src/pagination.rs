//! Pagination utilities for Phase 11
//!
//! Provides easy pagination for list endpoints with metadata.

use serde::{Deserialize, Serialize};
use crate::validation::{PaginationParams, PaginationMeta};

/// Generic paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The items for this page
    pub items: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationMeta,
}

impl<T> PaginatedResponse<T> {
    /// Create a paginated response
    pub fn new(items: Vec<T>, limit: u32, offset: u32, total: u32) -> Self {
        Self {
            items,
            pagination: PaginationMeta::new(limit, offset, total),
        }
    }

    /// Extract items for JSON serialization
    pub fn to_json(&self) -> String
    where
        T: Serialize,
    {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Helper to paginate a slice
pub fn paginate<T: Clone>(
    items: &[T],
    limit: u32,
    offset: u32,
) -> PaginatedResponse<T> {
    let limit = limit.min(100).max(1);  // 1-100
    let offset = offset as usize;
    let total = items.len() as u32;
    
    let start = offset.min(items.len());
    let end = (offset + limit as usize).min(items.len());
    
    let paged_items = items[start..end].to_vec();
    
    PaginatedResponse::new(paged_items, limit, offset as u32, total)
}

/// Helper to paginate a vector
pub fn paginate_vec<T: Clone>(
    items: Vec<T>,
    limit: u32,
    offset: u32,
) -> PaginatedResponse<T> {
    let limit = limit.min(100).max(1);  // 1-100
    let offset = offset as usize;
    let total = items.len() as u32;
    
    let start = offset.min(items.len());
    let end = (offset + limit as usize).min(items.len());
    
    let paged_items = items[start..end].to_vec();
    
    PaginatedResponse::new(paged_items, limit, offset as u32, total)
}

/// Helper to paginate with a filter function
pub fn paginate_filtered<T: Clone>(
    items: &[T],
    limit: u32,
    offset: u32,
    filter: impl Fn(&T) -> bool,
) -> PaginatedResponse<T> {
    let filtered: Vec<T> = items.iter().filter(|item| filter(item)).cloned().collect();
    paginate_vec(filtered, limit, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginate() {
        let items: Vec<i32> = (1..=100).collect();
        let response = paginate(&items, 20, 0);
        
        assert_eq!(response.items.len(), 20);
        assert_eq!(response.pagination.total, 100);
        assert_eq!(response.pagination.pages, 5);
        assert!(response.pagination.has_next);
    }

    #[test]
    fn test_paginate_last_page() {
        let items: Vec<i32> = (1..=100).collect();
        let response = paginate(&items, 20, 80);
        
        assert_eq!(response.items.len(), 20);
        assert!(!response.pagination.has_next);
        assert!(response.pagination.has_prev);
    }

    #[test]
    fn test_paginate_filtered() {
        let items: Vec<i32> = (1..=100).collect();
        let response = paginate_filtered(&items, 10, 0, |x| x % 2 == 0);
        
        assert_eq!(response.items.len(), 10);
        assert_eq!(response.pagination.total, 50);  // 50 even numbers
    }
}
