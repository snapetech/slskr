//! Request validation module for Phase 11
//! 
//! Provides common validation patterns for query parameters,
//! request bodies, and pagination.

use serde::{Deserialize, Serialize};

/// Standard pagination parameters
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct PaginationParams {
    /// Items per page (1-100, default: 20)
    pub limit: Option<u32>,
    /// Page number (0-indexed, default: 0)
    pub offset: Option<u32>,
}

impl PaginationParams {
    /// Validate and normalize pagination parameters
    pub fn normalize(&self) -> (u32, u32) {
        let limit = self.limit.unwrap_or(20).min(100).max(1);
        let offset = self.offset.unwrap_or(0);
        (limit, offset)
    }

    /// Calculate total pages
    pub fn pages(&self, total: u32) -> u32 {
        let (limit, _) = self.normalize();
        (total + limit - 1) / limit
    }

    /// Validate request is within bounds
    pub fn validate(&self, total: u32) -> Result<(), String> {
        let (limit, offset) = self.normalize();
        let pages = self.pages(total);
        
        if pages > 0 && offset >= pages {
            return Err(format!("offset {} >= total pages {}", offset, pages));
        }
        
        Ok(())
    }
}

/// Standard filter parameters for search-like endpoints
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilterParams {
    /// Search query
    pub q: Option<String>,
    /// Filter field
    pub filter: Option<String>,
    /// Filter value
    pub value: Option<String>,
}

impl FilterParams {
    /// Validate filter parameters
    pub fn validate(&self) -> Result<(), String> {
        if let Some(q) = &self.q {
            if q.is_empty() {
                return Err("query cannot be empty".to_string());
            }
            if q.len() > 1000 {
                return Err("query too long (max 1000 chars)".to_string());
            }
        }
        
        Ok(())
    }
}

/// Sorting parameters
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SortParams {
    /// Sort by field
    pub sort_by: Option<String>,
    /// Sort order (asc/desc)
    pub order: Option<SortOrder>,
}

impl SortParams {
    /// Validate sort parameters
    pub fn validate(&self, allowed_fields: &[&str]) -> Result<(), String> {
        if let Some(field) = &self.sort_by {
            if !allowed_fields.contains(&field.as_str()) {
                return Err(format!("invalid sort field: {}", field));
            }
        }
        Ok(())
    }
}

/// Complete query parameters for list endpoints
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListQuery {
    pub pagination: Option<PaginationParams>,
    pub filter: Option<FilterParams>,
    pub sort: Option<SortParams>,
}

impl ListQuery {
    /// Parse from individual params
    pub fn from_parts(
        limit: Option<u32>,
        offset: Option<u32>,
        q: Option<String>,
        sort_by: Option<String>,
        order: Option<SortOrder>,
    ) -> Self {
        Self {
            pagination: Some(PaginationParams { limit, offset }),
            filter: q.map(|query| FilterParams {
                q: Some(query),
                filter: None,
                value: None,
            }),
            sort: sort_by.map(|field| SortParams {
                sort_by: Some(field),
                order,
            }),
        }
    }

    /// Validate all parameters
    pub fn validate(&self, total: u32, allowed_fields: &[&str]) -> Result<(), String> {
        if let Some(p) = &self.pagination {
            p.validate(total)?;
        }
        if let Some(f) = &self.filter {
            f.validate()?;
        }
        if let Some(s) = &self.sort {
            s.validate(allowed_fields)?;
        }
        Ok(())
    }
}

/// Response metadata for paginated endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    /// Current page offset
    pub offset: u32,
    /// Items per page
    pub limit: u32,
    /// Total items available
    pub total: u32,
    /// Total pages
    pub pages: u32,
    /// Has next page
    pub has_next: bool,
    /// Has previous page
    pub has_prev: bool,
}

impl PaginationMeta {
    /// Create metadata from parameters
    pub fn new(limit: u32, offset: u32, total: u32) -> Self {
        let pages = (total + limit - 1) / limit;
        let current_page = if limit > 0 { offset / limit } else { 0 };
        
        Self {
            offset,
            limit,
            total,
            pages,
            has_next: current_page < pages - 1,
            has_prev: current_page > 0,
        }
    }
}

/// Validate integer ranges
pub fn validate_range(value: u32, min: u32, max: u32, name: &str) -> Result<(), String> {
    if value < min || value > max {
        Err(format!("{} must be between {} and {}, got {}", name, min, max, value))
    } else {
        Ok(())
    }
}

/// Validate string length
pub fn validate_string_length(value: &str, min: usize, max: usize, name: &str) -> Result<(), String> {
    if value.len() < min || value.len() > max {
        Err(format!("{} length must be between {} and {}", name, min, max))
    } else {
        Ok(())
    }
}

/// Validate required field
pub fn validate_required<T>(value: Option<T>, name: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("{} is required", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_normalize() {
        let params = PaginationParams { limit: Some(50), offset: Some(10) };
        let (limit, offset) = params.normalize();
        assert_eq!(limit, 50);
        assert_eq!(offset, 10);
    }

    #[test]
    fn test_pagination_max_limit() {
        let params = PaginationParams { limit: Some(500), offset: None };
        let (limit, _) = params.normalize();
        assert_eq!(limit, 100);  // capped at 100
    }

    #[test]
    fn test_filter_validation() {
        let f = FilterParams {
            q: Some("test".to_string()),
            filter: None,
            value: None,
        };
        assert!(f.validate().is_ok());
    }

    #[test]
    fn test_filter_empty_query() {
        let f = FilterParams {
            q: Some("".to_string()),
            filter: None,
            value: None,
        };
        assert!(f.validate().is_err());
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(20, 0, 50);
        assert_eq!(meta.total, 50);
        assert_eq!(meta.pages, 3);
        assert!(meta.has_next);
        assert!(!meta.has_prev);
    }
}
