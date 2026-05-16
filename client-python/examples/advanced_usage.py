#!/usr/bin/env python3
"""
Advanced usage example for slskr Python client
- Error handling
- Retries and timeouts
- Context managers
- Concurrent operations
"""

import asyncio
from slskr import SlskrClient, ApiError, NetworkError, TimeoutError


async def search_with_retry(client, query, max_retries=3):
    """Search with automatic retry on failure"""
    
    for attempt in range(max_retries):
        try:
            result = await client.create_search(query=query)
            print(f"✓ Search created: {result.get('id')}")
            return result
        except ApiError as e:
            if e.is_not_found():
                print(f"Search endpoint not found")
                return None
            elif e.is_unauthorized():
                print("Invalid API key")
                return None
            print(f"Attempt {attempt + 1} failed: {e}")
            if attempt < max_retries - 1:
                await asyncio.sleep(2 ** attempt)  # Exponential backoff
        except TimeoutError:
            print(f"Timeout on attempt {attempt + 1}")
            if attempt < max_retries - 1:
                await asyncio.sleep(1)
        except NetworkError as e:
            print(f"Network error: {e}")
            if attempt < max_retries - 1:
                await asyncio.sleep(2)
    
    return None


async def concurrent_searches(client, queries):
    """Execute multiple searches concurrently"""
    
    tasks = [
        search_with_retry(client, q)
        for q in queries
    ]
    
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    successful = [r for r in results if r is not None and not isinstance(r, Exception)]
    print(f"\nCompleted {len(successful)}/{len(queries)} searches")
    
    return successful


async def fetch_all_results(client, search_id, batch_size=50):
    """Fetch all search results with pagination"""
    
    all_results = []
    offset = 0
    
    while True:
        details = await client.get_search_details(search_id, limit=batch_size)
        results = details.get("results", [])
        
        if not results:
            break
        
        all_results.extend(results)
        offset += len(results)
        print(f"Fetched {offset} results...")
        
        # Stop if we got less than batch_size (indicates end of results)
        if len(results) < batch_size:
            break
    
    return all_results


async def main():
    """Advanced usage examples"""
    
    # Using context manager
    async with SlskrClient(
        base_url="http://127.0.0.1:5030",
        token="your-api-key-here",
        timeout=30,
        retries=3,
        debug=True
    ) as client:
        
        try:
            # Example 1: Single search with retry
            print("=== Example 1: Search with Retry ===")
            search = await search_with_retry(client, "debussy prelude")
            
            if search:
                search_id = search.get('id')
                
                # Example 2: Fetch all results with pagination
                print("\n=== Example 2: Paginated Results ===")
                all_results = await fetch_all_results(client, search_id, batch_size=50)
                print(f"Total results: {len(all_results)}")
                
                # Example 3: Process results
                if all_results:
                    print("\n=== Example 3: Process Results ===")
                    for result in all_results[:5]:
                        print(f"  {result.get('filename')} ({result.get('filesize')} bytes)")
            
            # Example 4: Concurrent operations
            print("\n=== Example 4: Concurrent Searches ===")
            queries = ["radiohead", "pink floyd", "david bowie", "the beatles"]
            searches = await concurrent_searches(client, queries)
            
            # Example 5: Batch operations
            print("\n=== Example 5: Batch Operations ===")
            batch = client.batch.builder()
            batch.get("/api/stats", op_id="stats")
            batch.get("/api/config", op_id="config")
            batch.get("/api/capabilities", op_id="caps")
            
            response = await batch.execute()
            print(f"Batch results: {len(response.get_successful())} successful")
            
            # Example 6: Error handling in batch
            print("\n=== Example 6: Error Handling ===")
            batch2 = client.batch.builder()
            batch2.get("/api/invalid-endpoint", op_id="invalid")
            batch2.get("/api/stats", op_id="valid")
            
            response2 = await batch2.execute()
            for result in response2.results:
                if result.is_error():
                    print(f"Operation {result.id} failed with status {result.status}")
                else:
                    print(f"Operation {result.id} succeeded")
            
        except Exception as e:
            print(f"Unexpected error: {e}")
            import traceback
            traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
