//! HTTP request pipelining for 500K req/sec throughput
//! Instead of request-response-request-response...
//! Send: request1, request2, request3 (batched)
//! Receive: response1, response2, response3 (in order)
//! Benefit: Amortize network round-trip time across multiple requests

use std::collections::VecDeque;
use tokio::sync::mpsc;

/// Pipelined request
#[derive(Debug, Clone)]
pub struct PipelinedRequest {
    pub id: u64,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Pipelined response  
#[derive(Debug, Clone)]
pub struct PipelinedResponse {
    pub id: u64,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Pipeline manager (FIFO queue of requests)
pub struct RequestPipeline {
    queue: VecDeque<PipelinedRequest>,
    max_queue_size: usize,
    request_id_counter: u64,
}

impl RequestPipeline {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_queue_size),
            max_queue_size,
            request_id_counter: 0,
        }
    }

    /// Enqueue request for pipelining
    pub fn enqueue(&mut self, method: String, path: String, body: Vec<u8>) -> Result<u64, String> {
        if self.queue.len() >= self.max_queue_size {
            return Err("Pipeline queue full".to_string());
        }

        let id = self.request_id_counter;
        self.request_id_counter += 1;

        self.queue.push_back(PipelinedRequest {
            id,
            method,
            path,
            headers: Vec::new(),
            body,
        });

        Ok(id)
    }

    /// Dequeue next request for processing
    pub fn dequeue(&mut self) -> Option<PipelinedRequest> {
        self.queue.pop_front()
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }

    /// Batch dequeue (process multiple at once)
    pub fn dequeue_batch(&mut self, max_batch: usize) -> Vec<PipelinedRequest> {
        let count = std::cmp::min(max_batch, self.queue.len());
        let mut batch = Vec::with_capacity(count);

        for _ in 0..count {
            if let Some(req) = self.queue.pop_front() {
                batch.push(req);
            }
        }

        batch
    }

    /// Drain entire queue
    pub fn drain_all(&mut self) -> Vec<PipelinedRequest> {
        self.queue.drain(..).collect()
    }
}

/// Response queue (in-order)
pub struct ResponseQueue {
    queue: VecDeque<PipelinedResponse>,
    next_expected_id: u64,
}

impl ResponseQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            next_expected_id: 0,
        }
    }

    /// Enqueue response (must be in order!)
    pub fn enqueue(&mut self, response: PipelinedResponse) -> Result<(), String> {
        if response.id != self.next_expected_id {
            return Err(format!(
                "Out of order response. Expected {}, got {}",
                self.next_expected_id, response.id
            ));
        }

        self.queue.push_back(response);
        self.next_expected_id += 1;
        Ok(())
    }

    /// Dequeue next response
    pub fn dequeue(&mut self) -> Option<PipelinedResponse> {
        self.queue.pop_front()
    }

    /// Get all ready responses
    pub fn drain_all(&mut self) -> Vec<PipelinedResponse> {
        self.queue.drain(..).collect()
    }
}

/// Pipelining metrics
#[derive(Debug, Clone)]
pub struct PipeliningStats {
    pub total_requests_pipelined: u64,
    pub total_responses_sent: u64,
    pub avg_pipeline_depth: f32,
    pub max_queue_size_observed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_pipelining() {
        let mut pipeline = RequestPipeline::new(100);

        // Enqueue multiple requests at once
        let id1 = pipeline.enqueue("GET".to_string(), "/api/health".to_string(), vec![]).unwrap();
        let id2 = pipeline.enqueue("GET".to_string(), "/api/stats".to_string(), vec![]).unwrap();
        let id3 = pipeline.enqueue("POST".to_string(), "/api/search".to_string(), vec![1, 2, 3]).unwrap();

        assert_eq!(pipeline.queue_size(), 3);
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
    }

    #[test]
    fn test_batch_dequeue() {
        let mut pipeline = RequestPipeline::new(100);

        for i in 0..10 {
            pipeline.enqueue(
                "GET".to_string(),
                format!("/api/endpoint/{}", i),
                vec![],
            ).ok();
        }

        let batch = pipeline.dequeue_batch(5);
        assert_eq!(batch.len(), 5);
        assert_eq!(pipeline.queue_size(), 5);
    }

    #[test]
    fn test_response_ordering() {
        let mut queue = ResponseQueue::new();

        let resp1 = PipelinedResponse {
            id: 0,
            status: 200,
            headers: vec![],
            body: b"resp1".to_vec(),
        };

        let resp2 = PipelinedResponse {
            id: 1,
            status: 200,
            headers: vec![],
            body: b"resp2".to_vec(),
        };

        queue.enqueue(resp1).ok();
        queue.enqueue(resp2).ok();

        let r1 = queue.dequeue().unwrap();
        let r2 = queue.dequeue().unwrap();

        assert_eq!(r1.id, 0);
        assert_eq!(r2.id, 1);
    }

    #[test]
    fn test_response_out_of_order_error() {
        let mut queue = ResponseQueue::new();

        let resp = PipelinedResponse {
            id: 5, // Out of order!
            status: 200,
            headers: vec![],
            body: vec![],
        };

        assert!(queue.enqueue(resp).is_err());
    }
}
