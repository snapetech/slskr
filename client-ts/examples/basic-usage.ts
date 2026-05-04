/**
 * Basic usage example for soulseekR API client
 */

import SoulseekrClient from '../src/client';
import { BatchClient } from '../src/batch-client';
import { WebSocketClient } from '../src/websocket-client';

// ============================================================================
// Basic REST API Usage
// ============================================================================

async function basicRestExample() {
  // Create client
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'your-bearer-token',
    timeout: 30000,
    debug: true,
  });

  try {
    // Check health
    const health = await client.health();
    console.log('Server health:', health);

    // Get version
    const version = await client.version();
    console.log('Server version:', version);

    // Get stats
    const stats = await client.getStats();
    console.log('Server stats:', stats);

    // List transfers
    const transfers = await client.listTransfers({
      direction: 'download',
      status: 'active',
      limit: 10,
    });
    console.log('Active downloads:', transfers);

    // Get specific transfer
    if (transfers.length > 0) {
      const transferDetails = await client.getTransfer(transfers[0].id);
      console.log('Transfer details:', transferDetails);
    }

    // Send message
    const message = await client.sendMessage({
      recipient: 'username',
      content: 'Hello!',
    });
    console.log('Message sent:', message);

    // Create search
    const search = await client.createSearch({
      query: 'artist name',
    });
    console.log('Search created:', search);

    // Get search results
    const searchDetails = await client.getSearchDetails(search.id, {
      limit: 20,
    });
    console.log('Search results:', searchDetails.results);
  } catch (error) {
    console.error('Error:', error);
  }
}

// ============================================================================
// Batch Operations
// ============================================================================

async function batchOperationsExample() {
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'your-bearer-token',
  });

  const batch = new BatchClient(client);

  try {
    // Execute batch operations
    const response = await batch
      .builder()
      .get('/api/stats', 'stats')
      .get('/api/transfers', 'transfers')
      .get('/api/messages', 'messages')
      .execute();

    console.log(`Batch completed in ${response.total_time_ms}ms`);
    console.log('Results:', response.results);

    // Check results
    if (batch.allSuccessful(response)) {
      console.log('All operations successful!');
    } else {
      const failed = batch.getFailed(response);
      console.error('Failed operations:', failed);
    }
  } catch (error) {
    console.error('Batch error:', error);
  }
}

// ============================================================================
// Bulk Message Send
// ============================================================================

async function bulkMessageExample() {
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'your-bearer-token',
  });

  const batch = new BatchClient(client);

  const recipients = ['user1', 'user2', 'user3'];
  const message = 'Hello everyone!';

  try {
    const response = await batch
      .builder()
      .post('/api/messages', { recipient: 'user1', content: message }, 'msg1')
      .post('/api/messages', { recipient: 'user2', content: message }, 'msg2')
      .post('/api/messages', { recipient: 'user3', content: message }, 'msg3')
      .execute();

    const successful = batch.getSuccessful(response);
    console.log(`Sent ${successful.length}/${recipients.length} messages`);
  } catch (error) {
    console.error('Error:', error);
  }
}

// ============================================================================
// WebSocket Real-Time Events
// ============================================================================

async function websocketExample() {
  const ws = new WebSocketClient('http://localhost:8080', 'your-bearer-token');

  // Listen to connection changes
  ws.onConnectionChange((connected) => {
    console.log(connected ? 'Connected to WebSocket' : 'Disconnected from WebSocket');
  });

  // Listen to errors
  ws.onError((error) => {
    console.error('WebSocket error:', error);
  });

  // Subscribe to transfer events
  ws.subscribe('transfer.started', 'transfer.completed', 'transfer.failed');

  // Listen to specific events
  ws.on('transfer.started', (event) => {
    console.log('Transfer started:', event.data);
  });

  ws.on('transfer.completed', (event) => {
    console.log('Transfer completed:', event.data);
  });

  ws.on('transfer.failed', (event) => {
    console.log('Transfer failed:', event.data);
  });

  try {
    // Connect
    await ws.connect();

    // Keep connection open
    setTimeout(() => {
      ws.disconnect();
    }, 60000); // 1 minute
  } catch (error) {
    console.error('Connection error:', error);
  }
}

// ============================================================================
// Pagination Example
// ============================================================================

async function paginationExample() {
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'your-bearer-token',
  });

  try {
    let offset = 0;
    const limit = 20;
    let hasMore = true;

    while (hasMore) {
      const messages = await client.listMessages({
        limit,
        offset,
      });

      console.log(`Page ${offset / limit + 1}: ${messages.length} messages`);

      hasMore = messages.length === limit;
      offset += limit;
    }
  } catch (error) {
    console.error('Error:', error);
  }
}

// ============================================================================
// Error Handling
// ============================================================================

async function errorHandlingExample() {
  const client = new SoulseekrClient({
    baseUrl: 'http://localhost:8080',
    token: 'your-bearer-token',
  });

  try {
    await client.getTransfer('non-existent-id');
  } catch (error: any) {
    if (error.status === 404) {
      console.error('Transfer not found');
    } else if (error.status === 401) {
      console.error('Invalid token');
    } else if (error.status === 500) {
      console.error('Server error');
    }
  }
}

// ============================================================================
// Run Examples
// ============================================================================

async function runExamples() {
  console.log('=== Basic REST API ===');
  await basicRestExample();

  console.log('\n=== Batch Operations ===');
  await batchOperationsExample();

  console.log('\n=== Bulk Messages ===');
  await bulkMessageExample();

  console.log('\n=== WebSocket Events ===');
  await websocketExample();

  console.log('\n=== Pagination ===');
  await paginationExample();

  console.log('\n=== Error Handling ===');
  await errorHandlingExample();
}

// Uncomment to run examples
// runExamples().catch(console.error);

export {
  basicRestExample,
  batchOperationsExample,
  bulkMessageExample,
  websocketExample,
  paginationExample,
  errorHandlingExample,
};
