#!/usr/bin/env python3
"""
Batch operations example for soulseekR Python client
"""

import asyncio
from soulseekr import SoulseekrClient


async def main():
    """Batch operations example"""
    
    client = SoulseekrClient(
        base_url="http://localhost:8080",
        token="your-api-key-here",
        debug=True
    )
    
    try:
        # Create batch of operations
        batch = client.batch.builder()
        
        # Add multiple GET operations
        batch.get("/api/stats", op_id="get-stats")
        batch.get("/api/capabilities", op_id="get-caps")
        batch.get("/api/config", op_id="get-config")
        
        # Add search creation operations
        batch.post(
            "/api/searches",
            {"query": "bach concerto"},
            op_id="search-1"
        )
        batch.post(
            "/api/searches",
            {"query": "mozart sonata"},
            op_id="search-2"
        )
        
        print(f"Executing batch of {batch.size()} operations...")
        
        # Execute batch
        response = await batch.execute()
        
        print(f"\nBatch completed in {response.total_time_ms}ms")
        print(f"Successful: {len(response.get_successful())}")
        print(f"Failed: {len(response.get_failed())}")
        
        # Process results
        for result in response.results:
            print(f"\nOperation: {result.id}")
            print(f"  Status: {result.status}")
            if result.is_success():
                print(f"  Result: {str(result.body)[:100]}...")
            else:
                print(f"  Error: {result.body}")
        
        # Demonstrate batch monitoring
        if response.all_successful():
            print("\n✓ All operations successful!")
        else:
            print(f"\n✗ {len(response.get_failed())} operations failed")
            
    except Exception as e:
        print(f"Error: {e}")
    finally:
        await client.close()


if __name__ == "__main__":
    asyncio.run(main())
